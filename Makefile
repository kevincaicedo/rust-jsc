# Define variables
IMAGE_NAME := javascriptcore

help:
	@echo "Usage: make [target] [platform=<platform>]"
	@echo "Targets:"
	@echo "  build-docker-jsc: Build the Docker image with JavaScriptCore"
	@echo "  build-jsc: Build JavaScriptCore"
	@echo "  build-lib: Build the Rust library"
	@echo "  gen-bindings: Generate the Rust bindings"
	@echo "  test: Run the tests"
	@echo "  all-tests: Run all the tests"
	@echo "  archive: Archive the build artifacts with the platform parameter"

test:
	RUST_BACKTRACE=1 cargo test --lib

all-tests:
	cargo test

run-example:
	(cd examples/hello_world && cargo run)

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

# Build the Docker image
build-docker-jsc-musl:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit" ]; then \
		git submodule update --init --recursive; \
	fi
# if .libs directory does not exist, create it
	@if [ ! -d ".libs-musl" ]; then \
		mkdir .libs-musl; \
	fi
	DOCKER_BUILDKIT=1 docker build -o ./.libs-musl -t "$(IMAGE_NAME)-musl" -f Dockerfile.musl .

# Build the Docker image
build-docker-jsc-arm:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit" ]; then \
		git submodule update --init --recursive; \
	fi
# if .libs directory does not exist, create it
	@if [ ! -d ".libs-arm" ]; then \
		mkdir .libs-arm; \
	fi
	DOCKER_BUILDKIT=1 docker build -o ./.libs-arm -t "$(IMAGE_NAME)-arm" -f Dockerfile.arm .

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
	WebKit/Tools/Scripts/build-jsc --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_EXPERIMENTAL_FEATURES=OFF -DUSE_THIN_ARCHIVES=OFF -DCMAKE_BUILD_TYPE=Release"

build-jsc-debug:
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
# WebKit/Tools/Scripts/build-jsc --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_EXPERIMENTAL_FEATURES=OFF -DUSE_THIN_ARCHIVES=OFF -DCMAKE_EXE_LINKER_FLAGS='-framework Foundation -framework CoreFoundation' -DCMAKE_SHARED_LINKER_FLAGS='-framework Foundation -framework CoreFoundation' -DCMAKE_CXX_FLAGS='-Wno-deprecated-declarations' -DCMAKE_BUILD_TYPE=Debug"
	WebKit/Tools/Scripts/build-jsc --jsc-only --debug --cmakeargs="-DENABLE_STATIC_JSC=ON -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_EXPERIMENTAL_FEATURES=OFF -DUSE_THIN_ARCHIVES=OFF -DCMAKE_BUILD_TYPE=Debug"

# (cd examples/hello_world && cargo build --release &> output.txt)
# archive all *.a files from JSOnly build receive the name libjsc-<platform>.a.gz, platforn is a parameter
archive:
	@echo "Archiving the build artifacts..."

	@if [ -z "$(platform)" ]; then \
		echo "Please provide the platform parameter"; \
		exit 1; \
	fi

	@cd WebKit/WebKitBuild/JSCOnly/Release/lib/ && \
	tar -czf libjsc-$(platform).a.gz *.a && \
	mv libjsc-$(platform).a.gz ../../../../../

archive-debug:
	@echo "Archiving the build artifacts..."

	@if [ -z "$(platform)" ]; then \
		echo "Please provide the platform parameter"; \
		exit 1; \
	fi

	@cd WebKit/WebKitBuild/JSCOnly/Debug/lib/ && \
	tar -czf libjsc-$(platform).a.gz *.a && \
	mv libjsc-$(platform).a.gz ../../../../../

archive-linux:
	@echo "Archiving the build artifacts..."

	@if [ -z "$(platform)" ]; then \
		echo "Please provide the platform parameter"; \
		exit 1; \
	fi

	@cd .libs && \
	tar -czf libjsc-$(platform).a.gz *.a && \
	mv libjsc-$(platform).a.gz ../

build-lib:
	cargo build --release

gen-bindings:
	(cd gen && cargo build --release)

.PHONY: build-docker-jsc build-jsc build-lib gen-bindings test archive
