/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

package software.amazon.smithy.rust.codegen.smithy

import io.kotest.matchers.shouldBe
import org.junit.jupiter.api.Test
import software.amazon.smithy.model.shapes.ShapeId
import software.amazon.smithy.rust.codegen.smithy.customizations.ClientCustomizations
import software.amazon.smithy.rust.codegen.smithy.customize.CombinedCodegenDecorator
import software.amazon.smithy.rust.codegen.smithy.customize.NoOpEventStreamSigningDecorator
import software.amazon.smithy.rust.codegen.smithy.customize.RequiredCustomizations
import software.amazon.smithy.rust.codegen.smithy.generators.client.FluentClientDecorator
import software.amazon.smithy.rust.codegen.testutil.asSmithyModel
import software.amazon.smithy.rust.codegen.testutil.generatePluginContext
import kotlin.io.path.createDirectory
import kotlin.io.path.writeText

class CodegenVisitorTest {
    @Test
    fun `baseline transform verify mixins removed`() {
        val model = """
            namespace com.example

            use aws.protocols#awsJson1_0

            @awsJson1_0
            @aws.api#service(sdkId: "Test", endpointPrefix: "differentPrefix")
            service Example {
                operations: [ BasicOperation ]
            }

            operation BasicOperation {
                input: Shape
            }

            @mixin
            structure SimpleMixin {
                name: String
            }

            structure Shape with [
                SimpleMixin
            ] {
                greeting: String
            }
        """.asSmithyModel(smithyVersion = "2.0")
        val (ctx, testDir) = generatePluginContext(model)
        testDir.resolve("src").createDirectory()
        testDir.resolve("src/main.rs").writeText("fn main() {}")
        val codegenDecorator =
            CombinedCodegenDecorator.fromClasspath(
                ctx,
                ClientCustomizations(),
                RequiredCustomizations(),
                FluentClientDecorator(),
                NoOpEventStreamSigningDecorator(),
            )
        val visitor = CodegenVisitor(ctx, codegenDecorator)
        val baselineModel = visitor.baselineTransform(model)
        baselineModel.getShapesWithTrait(ShapeId.from("smithy.api#mixin")).isEmpty() shouldBe true
    }
}
