# Define variables
IMAGE_NAME := javascriptcore

help:
	@echo "Usage: make [target]"
	@echo "Targets:"
	@echo "  build-docker-jsc: Build the Docker image with JavaScriptCore"
	@echo "  build-jsc: Build JavaScriptCore"
	@echo "  build-lib: Build the Rust library"
	@echo "  gen-bindings: Generate the Rust bindings"

test:
	RUST_BACKTRACE=1 cargo test --lib

all-tests:
	cargo test

# Build the Docker image
build-docker-jsc:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit" ]; then \
		git submodule update --init --recursive; \
	fi
# if .libs directory does not exist, create it
	@if [ ! -d ".libs" ]; then \
		mkdir .libs; \
	fi
	DOCKER_BUILDKIT=1 docker build -o ./.libs -t $(IMAGE_NAME) .

build-jsc:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit/Tools" ]; then \
		git submodule update --init --recursive; \
	fi
# if it is macOS, build with cmake, check if cmake is installed or install it with brew
	@if [ "$(shell uname)" = "Darwin" ]; then \
		if [ ! -x "$(shell command -v cmake)" ]; then \
			brew install cmake; \
		fi; \
	fi
	WebKit/Tools/Scripts/build-webkit --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF"

# (cd examples/hello_world && cargo build --release &> output.txt)

build-lib:
	cargo build --release

gen-bindings:
	(cd gen && cargo build --release)

.PHONY: build-docker-jsc build-jsc build-lib gen-bindings test
