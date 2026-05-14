#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

usage() {
    cat <<'EOF'
Usage: bash scripts/run-sanitizers.sh <command>

Commands:
  build-webkit-asan      Configure/build ASAN JSCOnly plus WebKit API test binaries.
  build-webkit-ubsan     Configure/build UBSAN JSCOnly plus WebKit API test binaries.
  webkit-asan            Run TestWTF/TestJavaScriptCore against ASAN JSCOnly.
  webkit-ubsan           Run TestWTF/TestJavaScriptCore against UBSAN JSCOnly.
  rust-asan              Run Rust tests with nightly ASAN against ASAN JSCOnly.
  rust-ubsan             Run Rust tests against UBSAN JSCOnly with Rust UB checks where available.
  all                    Run both WebKit API and Rust ASAN/UBSAN validation.

Useful environment:
  WEBKIT_DIR                         WebKit checkout. Default: WebKit
  JSC_GENERATOR                      CMake generator. Default: Ninja
  JSC_JOBS                           Parallel build jobs. Default: detected CPU count
  JSC_SANITIZER_BUILD_ROOT           Build root. Default: WebKit/WebKitBuild/RustJSC-Sanitizers
  JSC_SANITIZER_PROFILE              CMake build type. Default: Release
  JSC_SANITIZER_STATIC               Build static JSC archives. Default: ON
  JSC_SANITIZER_API_TARGETS          API test targets. Default: "TestWTF TestJavaScriptCore"
  JSC_SANITIZER_CMAKE_ARGS           Extra CMake arguments.
  RUST_JSC_SANITIZER_TOOLCHAIN       Rust toolchain for Rust sanitizer runs. Default: nightly
  RUST_JSC_SANITIZER_TARGET          Rust target triple. Default: rustc host
  RUST_JSC_SANITIZER_CARGO_ARGS      Cargo test package args. Default: --workspace
  RUST_JSC_SANITIZER_RUST_TEST_ARGS  Cargo test binary args. Default: --test-threads=1
EOF
}

truthy() {
    case "${1:-}" in
        1|true|TRUE|yes|YES|on|ON) return 0 ;;
        *) return 1 ;;
    esac
}

detect_jobs() {
    if command -v nproc >/dev/null 2>&1; then
        nproc
        return
    fi

    getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4
}

detect_rust_target() {
    rustc -vV | sed -n 's/^host: //p'
}

sanitizer_suffix() {
    case "$1" in
        address) echo "ASAN" ;;
        undefined) echo "UBSAN" ;;
        *) echo "Unknown" ;;
    esac
}

build_dir_for() {
    local sanitizer="$1"
    local suffix
    suffix="$(sanitizer_suffix "${sanitizer}")"

    if [[ "${JSC_SANITIZER_STATIC:-ON}" == "ON" ]]; then
        echo "${JSC_SANITIZER_BUILD_ROOT:-${WEBKIT_DIR:-WebKit}/WebKitBuild/RustJSC-Sanitizers}/JSCOnly/${JSC_SANITIZER_PROFILE:-Release}-${suffix}-Static"
    else
        echo "${JSC_SANITIZER_BUILD_ROOT:-${WEBKIT_DIR:-WebKit}/WebKitBuild/RustJSC-Sanitizers}/JSCOnly/${JSC_SANITIZER_PROFILE:-Release}-${suffix}"
    fi
}

configure_webkit_sanitizer() {
    local sanitizer="$1"
    local build_dir="$2"
    local webkit_dir="${WEBKIT_DIR:-WebKit}"
    local profile="${JSC_SANITIZER_PROFILE:-Release}"
    local generator="${JSC_GENERATOR:-Ninja}"
    local args=(
        -S "${webkit_dir}"
        -B "${build_dir}"
        -G "${generator}"
        -DPORT=JSCOnly
        "-DCMAKE_BUILD_TYPE=${profile}"
        -DSHOW_BINDINGS_GENERATION_PROGRESS=1
        -DDEVELOPER_MODE=ON
        -DENABLE_REMOTE_INSPECTOR=ON
        -DENABLE_FTL_JIT=ON
        -DENABLE_EXPERIMENTAL_FEATURES=OFF
        "-DENABLE_SANITIZERS=${sanitizer}"
    )

    if [[ "${JSC_SANITIZER_STATIC:-ON}" == "ON" ]]; then
        args+=(-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF)
    fi

    if [[ -n "${CC:-}" ]]; then
        args+=("-DCMAKE_C_COMPILER=${CC}")
    fi

    if [[ -n "${CXX:-}" ]]; then
        args+=("-DCMAKE_CXX_COMPILER=${CXX}")
    fi

    if [[ -n "${JSC_SANITIZER_CMAKE_ARGS:-}" ]]; then
        local extra_args=()
        read -r -a extra_args <<< "${JSC_SANITIZER_CMAKE_ARGS}"
        args+=("${extra_args[@]}")
    fi

    cmake "${args[@]}"
}

build_webkit_sanitizer() {
    local sanitizer="$1"
    local build_dir
    build_dir="$(build_dir_for "${sanitizer}")"

    if truthy "${SKIP_WEBKIT_SANITIZER_BUILD:-0}"; then
        echo "Skipping WebKit sanitizer build for ${sanitizer}; using ${build_dir}"
        return
    fi

    if [[ ! -f "${build_dir}/CMakeCache.txt" ]] || truthy "${JSC_SANITIZER_FORCE_CONFIGURE:-0}"; then
        configure_webkit_sanitizer "${sanitizer}" "${build_dir}"
    fi

    local jobs="${JSC_JOBS:-$(detect_jobs)}"
    cmake --build "${build_dir}" --target jsc --parallel "${jobs}"

    local api_targets=()
    read -r -a api_targets <<< "${JSC_SANITIZER_API_TARGETS:-TestWTF TestJavaScriptCore}"
    for target in "${api_targets[@]}"; do
        cmake --build "${build_dir}" --target "${target}" --parallel "${jobs}"
    done
}

run_with_sanitizer_env() {
    local sanitizer="$1"
    shift

    case "${sanitizer}" in
        address)
            env \
                ASAN_OPTIONS="${ASAN_OPTIONS:-detect_leaks=0:detect_stack_use_after_return=0:strict_string_checks=1:check_initialization_order=1}" \
                LSAN_OPTIONS="${LSAN_OPTIONS:-detect_leaks=0}" \
                "$@"
            ;;
        undefined)
            env \
                UBSAN_OPTIONS="${UBSAN_OPTIONS:-print_stacktrace=1:halt_on_error=1}" \
                "$@"
            ;;
        *)
            "$@"
            ;;
    esac
}

api_binary_path() {
    local build_dir="$1"
    local binary="$2"

    if [[ -x "${build_dir}/bin/TestWebKitAPI/${binary}" ]]; then
        echo "${build_dir}/bin/TestWebKitAPI/${binary}"
        return
    fi

    echo "${build_dir}/bin/${binary}"
}

run_webkit_api_tests() {
    local sanitizer="$1"
    local build_dir
    build_dir="$(build_dir_for "${sanitizer}")"

    build_webkit_sanitizer "${sanitizer}"

    local api_targets=()
    read -r -a api_targets <<< "${JSC_SANITIZER_API_TARGETS:-TestWTF TestJavaScriptCore}"
    for target in "${api_targets[@]}"; do
        local binary
        binary="$(api_binary_path "${build_dir}" "${target}")"
        if [[ ! -x "${binary}" ]]; then
            echo "Expected WebKit API test binary not found or not executable: ${binary}" >&2
            exit 1
        fi

        run_with_sanitizer_env "${sanitizer}" "${binary}"
    done
}

require_linux_target() {
    local target="$1"
    case "${target}" in
        *-linux-gnu) ;;
        *)
            echo "Rust sanitizer runs are currently configured for Linux GNU targets only; got ${target}" >&2
            exit 2
            ;;
    esac
}

run_rust_tests() {
    local sanitizer="$1"
    local build_dir
    build_dir="$(build_dir_for "${sanitizer}")"

    build_webkit_sanitizer "${sanitizer}"

    local lib_dir="${build_dir}/lib"
    if [[ ! -f "${lib_dir}/libJavaScriptCore.a" ]]; then
        echo "Expected sanitized static JavaScriptCore archive not found under ${lib_dir}" >&2
        exit 1
    fi
    lib_dir="$(cd "${lib_dir}" && pwd)"

    local toolchain="${RUST_JSC_SANITIZER_TOOLCHAIN:-nightly}"
    local target="${RUST_JSC_SANITIZER_TARGET:-$(detect_rust_target)}"
    require_linux_target "${target}"

    local cargo_args=()
    read -r -a cargo_args <<< "${RUST_JSC_SANITIZER_CARGO_ARGS:---workspace}"
    local rust_test_args=()
    read -r -a rust_test_args <<< "${RUST_JSC_SANITIZER_RUST_TEST_ARGS:---test-threads=1}"

    export RUST_JSC_BUILD_MODE=static
    export RUST_JSC_CUSTOM_BUILD_PATH="${lib_dir}"
    export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

    case "${sanitizer}" in
        address)
            export RUSTFLAGS="${RUSTFLAGS:-} -Zsanitizer=address"
            export RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -Zsanitizer=address"
            run_with_sanitizer_env address cargo "+${toolchain}" test "${cargo_args[@]}" -Zbuild-std --target "${target}" -- "${rust_test_args[@]}"
            ;;
        undefined)
            export RUSTFLAGS="${RUSTFLAGS:-} -Zub-checks=yes -C link-arg=-fsanitize=undefined"
            export RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -Zub-checks=yes -C link-arg=-fsanitize=undefined"
            run_with_sanitizer_env undefined cargo "+${toolchain}" test "${cargo_args[@]}" -Zbuild-std --target "${target}" -- "${rust_test_args[@]}"
            ;;
        *)
            echo "Unsupported sanitizer for Rust tests: ${sanitizer}" >&2
            exit 2
            ;;
    esac
}

command="${1:-}"
case "${command}" in
    -h|--help|help)
        usage
        ;;
    build-webkit-asan|build-asan)
        build_webkit_sanitizer address
        ;;
    build-webkit-ubsan|build-ubsan)
        build_webkit_sanitizer undefined
        ;;
    webkit-asan|test-webkit-api-asan)
        run_webkit_api_tests address
        ;;
    webkit-ubsan|test-webkit-api-ubsan)
        run_webkit_api_tests undefined
        ;;
    rust-asan|test-rust-asan)
        run_rust_tests address
        ;;
    rust-ubsan|test-rust-ubsan)
        run_rust_tests undefined
        ;;
    all)
        run_webkit_api_tests address
        run_rust_tests address
        run_webkit_api_tests undefined
        run_rust_tests undefined
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
