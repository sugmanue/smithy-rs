/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::ready;
use pin_project_lite::pin_project;
use tower::{Layer, Service};

use crate::{
    request::{FromParts, FromRequest},
    response::IntoResponse,
};

use super::{OperationError, OperationShape};

/// A [`Layer`] responsible for taking an operation [`Service`], accepting and returning Smithy
/// types and converting it into a [`Service`] taking and returning [`http`] types.
///
/// See [`Upgrade`].
#[derive(Debug, Clone)]
pub struct UpgradeLayer<Protocol, Operation, Exts, B> {
    _protocol: PhantomData<Protocol>,
    _operation: PhantomData<Operation>,
    _exts: PhantomData<Exts>,
    _body: PhantomData<B>,
}

impl<P, Op, E, B> Default for UpgradeLayer<P, Op, E, B> {
    fn default() -> Self {
        Self {
            _protocol: PhantomData,
            _operation: PhantomData,
            _exts: PhantomData,
            _body: PhantomData,
        }
    }
}

impl<Protocol, Operation, Exts, B> UpgradeLayer<Protocol, Operation, Exts, B> {
    /// Creates a new [`UpgradeLayer`].
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, P, Op, E, B> Layer<S> for UpgradeLayer<P, Op, E, B> {
    type Service = Upgrade<P, Op, E, B, S>;

    fn layer(&self, inner: S) -> Self::Service {
        Upgrade {
            _protocol: PhantomData,
            _operation: PhantomData,
            _body: PhantomData,
            _exts: PhantomData,
            inner,
        }
    }
}

/// An alias allowing for quick access to [`UpgradeLayer`]s target [`Service`].
pub type UpgradedService<P, Op, E, B, S> = <UpgradeLayer<P, Op, E, B> as Layer<S>>::Service;

/// A [`Service`] responsible for wrapping an operation [`Service`] accepting and returning Smithy
/// types, and converting it into a [`Service`] accepting and returning [`http`] types.
pub struct Upgrade<Protocol, Operation, Exts, B, S> {
    _protocol: PhantomData<Protocol>,
    _operation: PhantomData<Operation>,
    _exts: PhantomData<Exts>,
    _body: PhantomData<B>,
    inner: S,
}

impl<P, Op, E, B, S> Clone for Upgrade<P, Op, E, B, S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            _protocol: PhantomData,
            _operation: PhantomData,
            _body: PhantomData,
            _exts: PhantomData,
            inner: self.inner.clone(),
        }
    }
}

pin_project! {
    #[project = InnerProj]
    #[project_replace = InnerProjReplace]
    enum Inner<FromFut, HandlerFut> {
        FromRequest {
            #[pin]
            inner: FromFut
        },
        Inner {
            #[pin]
            call: HandlerFut
        }
    }
}

type InnerAlias<Input, Exts, Protocol, B, Fut> = Inner<<(Input, Exts) as FromRequest<Protocol, B>>::Future, Fut>;

pin_project! {
    /// The [`Service::Future`] of [`Upgrade`].
    pub struct UpgradeFuture<Protocol, Operation, Exts, B, S>
    where
        Operation: OperationShape,
        (Operation::Input, Exts): FromRequest<Protocol, B>,
        S: Service<(Operation::Input, Exts)>,
    {
        service: S,
        #[pin]
        inner: InnerAlias<Operation::Input, Exts, Protocol, B, S::Future>
    }
}

impl<P, Op, Exts, B, S, PollError, OpError> Future for UpgradeFuture<P, Op, Exts, B, S>
where
    // `Op` is used to specify the operation shape
    Op: OperationShape,
    // Smithy input must convert from a HTTP request
    Op::Input: FromRequest<P, B>,
    // Smithy output must convert into a HTTP response
    Op::Output: IntoResponse<P>,
    // Smithy error must convert into a HTTP response
    OpError: IntoResponse<P>,

    // Must be able to convert extensions
    Exts: FromParts<P>,

    // The signature of the inner service is correct
    S: Service<(Op::Input, Exts), Response = Op::Output, Error = OperationError<OpError, PollError>>,
{
    type Output = Result<http::Response<crate::body::BoxBody>, PollError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut this = self.as_mut().project();
            let this2 = this.inner.as_mut().project();

            let call = match this2 {
                InnerProj::FromRequest { inner } => {
                    let result = ready!(inner.poll(cx));
                    match result {
                        Ok(ok) => this.service.call(ok),
                        Err(err) => return Poll::Ready(Ok(err.into_response())),
                    }
                }
                InnerProj::Inner { call } => {
                    let result = ready!(call.poll(cx));
                    let output = match result {
                        Ok(ok) => ok.into_response(),
                        Err(OperationError::Model(err)) => err.into_response(),
                        Err(OperationError::PollReady(_)) => {
                            unreachable!("poll error should not be raised")
                        }
                    };
                    return Poll::Ready(Ok(output));
                }
            };

            this.inner.as_mut().project_replace(Inner::Inner { call });
        }
    }
}

impl<P, Op, Exts, B, S, PollError, OpError> Service<http::Request<B>> for Upgrade<P, Op, Exts, B, S>
where
    // `Op` is used to specify the operation shape
    Op: OperationShape,
    // Smithy input must convert from a HTTP request
    Op::Input: FromRequest<P, B>,
    // Smithy output must convert into a HTTP response
    Op::Output: IntoResponse<P>,
    // Smithy error must convert into a HTTP response
    OpError: IntoResponse<P>,

    // Must be able to convert extensions
    Exts: FromParts<P>,

    // The signature of the inner service is correct
    S: Service<(Op::Input, Exts), Response = Op::Output, Error = OperationError<OpError, PollError>> + Clone,
{
    type Response = http::Response<crate::body::BoxBody>;
    type Error = PollError;
    type Future = UpgradeFuture<P, Op, Exts, B, S>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|err| match err {
            OperationError::PollReady(err) => err,
            OperationError::Model(_) => unreachable!("operation error should not be raised"),
        })
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        UpgradeFuture {
            service: self.inner.clone(),
            inner: Inner::FromRequest {
                inner: <(Op::Input, Exts) as FromRequest<P, B>>::from_request(req),
            },
        }
    }
}
