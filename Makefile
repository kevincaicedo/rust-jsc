# Define variables
IMAGE_NAME := javascriptcore

# Build the Docker image
build-jsc:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit" ]; then \
		git submodule update --init --recursive; \
	fi
# if .libs directory does not exist, create it
	@if [ ! -d ".libs" ]; then \
		mkdir .libs; \
	fi
	DOCKER_BUILDKIT=1 docker build -o ./.libs -t $(IMAGE_NAME) .

# (cd examples/hello_world && cargo build --release &> output.txt)

build-lib:
	cargo build --release

generate-bindings:
	(cd gen && cargo build --release)

.PHONY: build-jsc build-lib
