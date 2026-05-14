# Define variables
IMAGE_NAME := javascriptcore
AR ?= ar
WEBKIT_DIR ?= WebKit
JSC_PROFILE ?= Release
JSC_JOBS ?= $(shell sysctl -n hw.logicalcpu 2>/dev/null || nproc 2>/dev/null || echo 4)
JSC_GENERATOR ?= Ninja
JSC_STATIC ?= ON
JSC_BUILD_ROOT ?= $(WEBKIT_DIR)/WebKitBuild/RustJSC
JSC_BUILD_VARIANT = $(if $(filter ON,$(JSC_STATIC)),$(JSC_PROFILE)-Static,$(JSC_PROFILE))
JSC_BUILD_DIR ?= $(JSC_BUILD_ROOT)/JSCOnly/$(JSC_BUILD_VARIANT)
LOCAL_JSC_LIB_DIR ?= $(JSC_BUILD_DIR)/lib
LOCAL_JSC_LIB_DIR_ABS = $(abspath $(LOCAL_JSC_LIB_DIR))
JSC_EXTRA_CMAKE_ARGS ?=
JSC_CMAKE_ARGS := -DPORT=JSCOnly -DCMAKE_BUILD_TYPE=$(JSC_PROFILE) -DSHOW_BINDINGS_GENERATION_PROGRESS=1 -DDEVELOPER_MODE=ON -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_FTL_JIT=ON $(JSC_EXTRA_CMAKE_ARGS)
JSC_SANITIZER_BUILD_ROOT ?= $(WEBKIT_DIR)/WebKitBuild/RustJSC-Sanitizers
JSC_SANITIZER_PROFILE ?= Release
SANITIZER_ENV = WEBKIT_DIR="$(WEBKIT_DIR)" JSC_GENERATOR="$(JSC_GENERATOR)" JSC_JOBS="$(JSC_JOBS)" JSC_SANITIZER_BUILD_ROOT="$(JSC_SANITIZER_BUILD_ROOT)" JSC_SANITIZER_PROFILE="$(JSC_SANITIZER_PROFILE)" JSC_SANITIZER_CMAKE_ARGS="$(JSC_EXTRA_CMAKE_ARGS)"

ifeq ($(JSC_STATIC),ON)
JSC_CMAKE_ARGS += -DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF
endif

help:
	@echo "Usage: make [target] [platform=<platform>]"
	@echo "Targets:"
	@echo "  build-docker-jsc: Build the Docker image with JavaScriptCore"
	@echo "  build-jsc: Build JavaScriptCore static archives with direct CMake/JSCOnly"
	@echo "  build-jsc-static: Alias for the default static JSC build"
	@echo "  jsc-jit-archive: Create libJavaScriptCoreJIT.a from CMake JIT objects when present"
	@echo "  build-jsc-buildjsc: Build JavaScriptCore through WebKit/Tools/Scripts/build-jsc"
	@echo "  jsc-smoke: Run a small smoke test against the local jsc binary"
	@echo "  build-lib: Build the Rust library"
	@echo "  build-lib-local-jsc: Build Rust library against the local JSC build"
	@echo "  gen-bindings: Generate the Rust bindings"
	@echo "  test: Run the unit tests"
	@echo "  test-local-jsc: Run unit tests against the local JSC build"
	@echo "  test-webkit-api-asan: Build ASAN JSCOnly and run TestWTF/TestJavaScriptCore"
	@echo "  test-webkit-api-ubsan: Build UBSAN JSCOnly and run TestWTF/TestJavaScriptCore"
	@echo "  test-rust-asan: Run Rust tests with nightly ASAN against ASAN JSCOnly"
	@echo "  test-rust-ubsan: Run Rust tests against UBSAN JSCOnly with Rust UB checks where available"
	@echo "  test-sanitizers: Run practical WebKit API and Rust ASAN/UBSAN validation"
	@echo "  all-tests: Run all the tests (including workspace members)"
	@echo "  archive: Archive the build artifacts with the platform parameter"
	@echo "  bench: Run all Criterion benchmarks"
	@echo "  bench-report: Run benchmarks and open HTML report"
	@echo "  run-stress: Run the comprehensive stress example"
	@echo "  profile-heap: Run DHAT heap profiling on the profiling tool"
	@echo "  flamegraph: Generate flamegraph from the stress example"

test:
	RUST_BACKTRACE=1 cargo test --lib -- --test-threads=1

all-tests:
	cargo test

run-example:
	(cd examples/hello_world && cargo run)

run-stress:
	cargo run --manifest-path examples/stress/Cargo.toml --release

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
	cmake -S $(WEBKIT_DIR) -B $(JSC_BUILD_DIR) -G "$(JSC_GENERATOR)" $(JSC_CMAKE_ARGS)
	cmake --build $(JSC_BUILD_DIR) --target jsc --parallel $(JSC_JOBS)
	@if [ "$(JSC_STATIC)" = "ON" ]; then \
		$(MAKE) jsc-jit-archive JSC_BUILD_DIR=$(JSC_BUILD_DIR); \
	fi

build-jsc-static:
	$(MAKE) build-jsc JSC_STATIC=ON JSC_BUILD_DIR=$(JSC_BUILD_ROOT)/JSCOnly/$(JSC_PROFILE)-Static

jsc-jit-archive:
	@if [ ! -d "$(JSC_BUILD_DIR)/Source/JavaScriptCore/CMakeFiles/JavaScriptCoreJIT.dir" ]; then \
		echo "JavaScriptCoreJIT object directory not produced for $(JSC_BUILD_DIR); libJavaScriptCore.a is self-contained for this target"; \
		$(RM) "$(LOCAL_JSC_LIB_DIR)/libJavaScriptCoreJIT.a"; \
	else \
		mkdir -p "$(LOCAL_JSC_LIB_DIR)"; \
		$(RM) "$(LOCAL_JSC_LIB_DIR)/libJavaScriptCoreJIT.a"; \
		find "$(JSC_BUILD_DIR)/Source/JavaScriptCore/CMakeFiles/JavaScriptCoreJIT.dir" -type f -name '*.o' -exec "$(AR)" rcs "$(LOCAL_JSC_LIB_DIR)/libJavaScriptCoreJIT.a" {} +; \
	fi

build-jsc-debug:
# Check if WebKit submodule is initialized otherwise initialize it
	@if [ ! -d "WebKit/Tools" ]; then \
		git submodule update --init --recursive; \
	fi
	$(MAKE) build-jsc JSC_PROFILE=Debug JSC_BUILD_DIR=$(JSC_BUILD_ROOT)/JSCOnly/Debug-Static

build-jsc-buildjsc:
	WebKit/Tools/Scripts/build-jsc --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_EXPERIMENTAL_FEATURES=OFF -DCMAKE_BUILD_TYPE=$(JSC_PROFILE)"
	$(MAKE) jsc-jit-archive JSC_BUILD_DIR=$(WEBKIT_DIR)/WebKitBuild/JSCOnly/$(JSC_PROFILE) LOCAL_JSC_LIB_DIR=$(WEBKIT_DIR)/WebKitBuild/JSCOnly/$(JSC_PROFILE)/lib

jsc-smoke:
	$(JSC_BUILD_DIR)/bin/jsc -e "print(1 + 1)"

# Archive all *.a files from a JSCOnly build as libjsc-<platform>.a.gz.
# The packager writes deterministic tar.gz bytes plus .sha256 and metadata JSON.
archive:
	@echo "Archiving the build artifacts..."

	@if [ -z "$(platform)" ]; then \
		echo "Please provide the platform parameter"; \
		exit 1; \
	fi

	@if [ "$(JSC_STATIC)" = "ON" ]; then \
		$(MAKE) jsc-jit-archive JSC_BUILD_DIR=$(JSC_BUILD_DIR); \
	fi

	@if [ ! -d "$(LOCAL_JSC_LIB_DIR)" ]; then \
		echo "Static library directory not found: $(LOCAL_JSC_LIB_DIR)"; \
		exit 1; \
	fi

	python3 scripts/package_jsc_archive.py --lib-dir "$(LOCAL_JSC_LIB_DIR)" --target-triple "$(platform)" --output-dir "$(CURDIR)" --repo-root "$(CURDIR)" --webkit-dir "$(WEBKIT_DIR)" --build-dir "$(JSC_BUILD_DIR)"

archive-debug:
	$(MAKE) archive platform=$(platform) JSC_PROFILE=Debug JSC_BUILD_DIR=$(JSC_BUILD_ROOT)/JSCOnly/Debug-Static

archive-linux:
	@echo "Archiving the build artifacts..."

	@if [ -z "$(platform)" ]; then \
		echo "Please provide the platform parameter"; \
		exit 1; \
	fi

	python3 scripts/package_jsc_archive.py --lib-dir ".libs" --target-triple "$(platform)" --output-dir "$(CURDIR)" --repo-root "$(CURDIR)" --webkit-dir "$(WEBKIT_DIR)"

build-lib:
	cargo build --release

build-lib-local-jsc:
	DYLD_FRAMEWORK_PATH="$(LOCAL_JSC_LIB_DIR_ABS):$$DYLD_FRAMEWORK_PATH" LD_LIBRARY_PATH="$(LOCAL_JSC_LIB_DIR_ABS):$$LD_LIBRARY_PATH" RUST_JSC_BUILD_MODE=download RUST_JSC_LIB_DIR="$(LOCAL_JSC_LIB_DIR_ABS)" cargo build --release

test-local-jsc:
	DYLD_FRAMEWORK_PATH="$(LOCAL_JSC_LIB_DIR_ABS):$$DYLD_FRAMEWORK_PATH" LD_LIBRARY_PATH="$(LOCAL_JSC_LIB_DIR_ABS):$$LD_LIBRARY_PATH" RUST_JSC_BUILD_MODE=download RUST_JSC_LIB_DIR="$(LOCAL_JSC_LIB_DIR_ABS)" RUST_BACKTRACE=1 cargo test --lib -- --test-threads=1

build-jsc-asan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh build-webkit-asan

build-jsc-ubsan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh build-webkit-ubsan

test-webkit-api-asan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh webkit-asan

test-webkit-api-ubsan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh webkit-ubsan

test-rust-asan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh rust-asan

test-rust-ubsan:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh rust-ubsan

test-sanitizers:
	$(SANITIZER_ENV) bash scripts/run-sanitizers.sh all

gen-bindings:
	(cd gen && cargo build --release)

bench:
	cargo bench --bench rust_jsc_bench

bench-report:
	cargo bench --bench rust_jsc_bench
	@echo "Opening benchmark report..."
	@open target/criterion/report/index.html 2>/dev/null || xdg-open target/criterion/report/index.html 2>/dev/null || echo "Open target/criterion/report/index.html in your browser"

profile-heap:
	cargo run --manifest-path examples/profiling/Cargo.toml --release
	@echo "DHAT profile written to dhat-heap.json"
	@echo "View at: https://nnethercote.github.io/dh_view/dh_view.html"

flamegraph:
	cargo flamegraph --root --manifest-path examples/stress/Cargo.toml --release -o flamegraph.svg
	@echo "Flamegraph written to flamegraph.svg"

.PHONY: build-docker-jsc build-jsc build-jsc-static jsc-jit-archive build-jsc-debug build-jsc-buildjsc jsc-smoke build-lib build-lib-local-jsc gen-bindings test test-local-jsc build-jsc-asan build-jsc-ubsan test-webkit-api-asan test-webkit-api-ubsan test-rust-asan test-rust-ubsan test-sanitizers archive bench bench-report run-stress profile-heap flamegraph
