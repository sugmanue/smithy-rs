SRC_DIR := $(shell git rev-parse --show-toplevel)
CUR_DIR := $(shell pwd)
GRADLE := $(SRC_DIR)/gradlew
SERVER_SDK_DST := $(CUR_DIR)/pokemon-service-server-sdk
CLIENT_SDK_DST := $(CUR_DIR)/pokemon-service-client
SERVER_SDK_SRC := $(SRC_DIR)/codegen-server-test/build/smithyprojections/codegen-server-test/pokemon-service-server-sdk/rust-server-codegen
CLIENT_SDK_SRC := $(SRC_DIR)/codegen-test/build/smithyprojections/codegen-test/pokemon-service-client/rust-codegen

all: codegen

codegen:
	$(GRADLE) --project-dir $(SRC_DIR) -P modules='pokemon-service-server-sdk,pokemon-service-client' :codegen-test:assemble :codegen-server-test:assemble
	mkdir -p $(SERVER_SDK_DST) $(CLIENT_SDK_DST)
	cp -av $(SERVER_SDK_SRC)/* $(SERVER_SDK_DST)/
	cp -av $(CLIENT_SDK_SRC)/* $(CLIENT_SDK_DST)/

build: codegen
	cargo build

run: codegen
	cargo run

doc-open: codegen
	cargo doc --no-deps --open

clean:
	cargo clean || echo "Unable to run cargo clean"

lambda_watch:
	cargo lambda watch

lambda_invoke:
	cargo lambda invoke pokemon-service-lambda --data-file pokemon-service/tests/fixtures/example-apigw-request.json

distclean: clean
	rm -rf $(SERVER_SDK_DST) $(CLIENT_SDK_DST) Cargo.lock

.PHONY: all
