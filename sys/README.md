# rust-jsc-sys Build Configuration

`rust_jsc_sys` provides the raw JavaScriptCore FFI bindings and the Cargo build
script used by `rust_jsc`. It links against artifacts produced from the Kedo
WebKit fork, not stock JavaScriptCore, because rust-jsc depends on fork-only
APIs for modules, synthetic modules, inspector integration, shared data, typed
arrays, and error helpers.

This document is the build and linking reference. The safe module-loader API,
JSON import-attribute behavior, WebAssembly modules, synthetic modules, and
host-pump guidance live in the
[rust-jsc module guide](https://github.com/kevincaicedo/rust-jsc/blob/main/docs/modules.md).
At the raw sys layer, legacy `JSModuleLoaderFetch` receives `attributesValue`
as `"json"`, `"javascript"`, `"webassembly"`, or `undefined` when
JavaScriptCore cannot provide a type. Binary or typed module loaders should use
`JSModuleLoaderFetchSource` and return `JSModuleSourceRef`. Hosts that load
WebAssembly modules may also need to call `JSRunDeferredWork` at an event-loop
checkpoint before draining microtasks.

## Raw API Contracts Added By The Kedo Fork

The sys crate is intentionally thin. Prefer the safe `rust_jsc` wrappers unless
you are writing a low-level bridge or validating headers.

| Raw API | Contract |
| --- | --- |
| `JSObjectGetArrayLength` | Exact JavaScript Array only. Returns `false` and stores a TypeError for proxies, array-like objects, and non-arrays. |
| `JSObjectArrayPush` | Exact JavaScript Array only. Uses JavaScriptCore array push semantics for one value and returns the new length. |
| `JSObjectCallMethod` | Performs observable property lookup, verifies the property is callable, calls with the original object as `this`, and returns a JS exception on failure. |
| `JSValueCreateUTF8ArrayBuffer` | Converts a JS value to UTF-8 string data and returns an ArrayBuffer containing those bytes. |
| Typed-array and ArrayBuffer byte helpers | Return borrowed JavaScriptCore storage pointers valid only while the backing object remains alive and not detached. Invalid object inputs report TypeError. |
| Inspector callback APIs | Message callbacks receive `(const char*, size_t)` borrowed for the callback duration only. Pause callbacks are pump/queue notifications on the JSC thread. |
| Shared-data APIs | WebKit stores a non-owning opaque pointer; Rust owns allocation, replacement, and drop. |

## Default Contract

The default is intentionally simple:

```text
RUST_JSC_BUILD_MODE=download
```

With no environment variables, `sys/build.rs` downloads the prebuilt static
archive for the current target from the rust-jsc GitHub release mirror, verifies
the exact matching SHA-256 manifest row, rejects unsafe archive paths, extracts
it into Cargo's build output directory, and links it statically.

There are three supported modes:

| Mode | Use when | Behavior |
| --- | --- | --- |
| `download` | Normal user, CI, release, or local prebuilt-static use. | Links an exact `RUST_JSC_ARCHIVE` override when set, then `RUST_JSC_LIB_DIR` when set, otherwise downloads `libjsc-<target>.a.gz` from the mirror. |
| `source` | You want Cargo to build JavaScriptCore from the bundled WebKit checkout. | Runs direct CMake/JSCOnly with Ninja and static JSC enabled. |
| `system` | You are experimenting with a system or dynamic JavaScriptCore. | Links a system library/framework. This is not the normal rust-jsc path and may miss fork-only APIs. |

Legacy mode strings `auto`, `custom`, `static`, and `archive` still map to
`download` during the 1.0 migration window. Legacy `framework` maps to `system`.

## Release Archive Contract

Release archives are static archive bundles named:

```text
libjsc-<target>.a.gz
```

Supported archive targets:

| Target | Archive |
| --- | --- |
| macOS x86_64 | `libjsc-x86_64-apple-darwin.a.gz` |
| macOS aarch64 | `libjsc-aarch64-apple-darwin.a.gz` |
| Linux glibc x86_64 | `libjsc-x86_64-unknown-linux-gnu.a.gz` |
| Linux glibc aarch64 | `libjsc-aarch64-unknown-linux-gnu.a.gz` |
| Linux musl x86_64 | `libjsc-x86_64-unknown-linux-musl.a.gz` |
| Linux musl aarch64 | `libjsc-aarch64-unknown-linux-musl.a.gz` |

Every archive must include:

```text
libJavaScriptCore.a
libWTF.a
libbmalloc.a
```

Some JSCOnly builds also produce:

```text
libJavaScriptCoreJIT.a
```

When present, `rust_jsc_sys` links it after `libJavaScriptCore.a`. When it is
absent, `libJavaScriptCore.a` is expected to be self-contained for that target.

Linux release archives also include the static system dependency archives used
by the Docker release builds:

```text
libstdc++.a
libicui18n.a
libicuuc.a
libicudata.a
libatomic.a
```

Local Linux source builds may not have those system archives beside JSC. In that
case the build script links JSC/WTF statically and falls back to dynamic host
`stdc++`, ICU, and `atomic` libraries. Release Docker archives remain fully
bundled for the default download path.

Every release also publishes:

```text
SHA256SUMS
libjsc-<target>.a.gz.sha256
libjsc-<target>.metadata.json
```

`sys/build.rs` downloads `SHA256SUMS` for default mirror downloads and verifies
the selected archive before extraction. Exact `RUST_JSC_ARCHIVE` overrides use a
sidecar named `<archive>.sha256` or a `SHA256SUMS` file beside the local archive.
The metadata JSON records the archive hash, included library hashes and sizes,
WebKit commit, rust-jsc sys version, target triple, repository commit, and
compiler/tool versions. Metadata is release evidence; it is not extracted or
linked by Cargo.

## Variables By Category

### Mode Selection

| Variable | Default | Description |
| --- | --- | --- |
| `RUST_JSC_BUILD_MODE` | `download` | Selects `download`, `source`, or `system`. |
| `RUST_JSC_FROM_SOURCE` | unset | Truthy values (`1`, `true`, `yes`, `on`) force `source` mode. This is a stable alias. |

### Download And Static Archive Inputs

| Variable | Default | Description |
| --- | --- | --- |
| `RUST_JSC_ARCHIVE` | unset | Specific archive file, `file://` URL, or `http(s)` URL to use instead of the default target archive URL. This takes precedence over `RUST_JSC_LIB_DIR`. |
| `RUST_JSC_LIB_DIR` | unset | Directory containing already-extracted static JSC archives. Use this for local Docker output or `make build-jsc-static` output. |
| `RUST_JSC_MIRROR` | GitHub release mirror | Mirror base URL. The build script appends `/sys-v<version>/libjsc-<target>.a.gz`. |

### Source Build Inputs

Source mode always uses direct CMake/JSCOnly, Ninja, and static JSC archives.
There is no public generator setting and no dynamic/framework source mode.

| Variable | Default | Description |
| --- | --- | --- |
| `RUST_JSC_WEBKIT_DIR` | `../WebKit` from the `sys` crate | WebKit checkout to build. |
| `RUST_JSC_BUILD_DIR` | `WebKit/WebKitBuild/RustJSC/JSCOnly/<profile>-Static` | CMake build directory. |
| `RUST_JSC_BUILD_PROFILE` | `Release` | CMake build type. Supported values: `Release`, `Debug`. |
| `RUST_JSC_JOBS` | host parallelism or `4` | Parallel CMake build jobs. |
| `RUST_JSC_ARCHIVE_OUT` | unset | Optional `.tar.gz` output path. When set, source mode archives the built static `.a` files after the build. |
| `RUST_JSC_CMAKE_ARGS` | unset | Advanced escape hatch for extra CMake `-D...` arguments. Keep this empty unless a build investigation needs it. |
| `RUST_JSC_CMAKE_GENERATOR` | `Ninja` | Compatibility input only. `Ninja` is accepted, any other value warns, and source mode still uses Ninja. |

### System Mode Inputs

System mode is for experiments only. Distribution JavaScriptCore builds usually
do not export rust-jsc's fork-only APIs.

| Variable | Default | Description |
| --- | --- | --- |
| `RUST_JSC_SYSTEM_LIBS_PATH` | unset | Extra native library search directory. |
| `RUST_JSC_SYSTEM_LIB_NAME` | `JavaScriptCore` | Dynamic library name to link. On macOS, `JavaScriptCore` links as a framework. |
| `SYSTEM_LIBS_PATH` | `/usr/lib` on macOS static links | Extra search path for macOS system libraries used by static archive links. |

### Legacy Aliases

These still work during the 1.0 migration window, but new scripts and docs
should use the variables above.

| Legacy | Replacement |
| --- | --- |
| `RUST_JSC_BUILD_MODE=auto` | `RUST_JSC_BUILD_MODE=download` |
| `RUST_JSC_BUILD_MODE=custom` | `RUST_JSC_BUILD_MODE=download` with `RUST_JSC_LIB_DIR` |
| `RUST_JSC_BUILD_MODE=static` | `RUST_JSC_BUILD_MODE=download` |
| `RUST_JSC_BUILD_MODE=archive` | `RUST_JSC_BUILD_MODE=download` |
| `RUST_JSC_BUILD_MODE=framework` | `RUST_JSC_BUILD_MODE=system` for dynamic/framework experiments |
| `RUST_JSC_FRAMEWORK_PATH` | system-mode macOS framework path; ignored by explicit `download` or `source` mode |
| `RUST_JSC_CUSTOM_BUILD_PATH` | `RUST_JSC_LIB_DIR` |
| `RUST_JSC_CUSTOM_ARCHIVE` | `RUST_JSC_ARCHIVE` |
| `RUST_JSC_CMAKE_GENERATOR` | compatibility-only source input; source mode always uses Ninja |
| `RUST_JSC_STATIC` | ignored; source mode is static by default |

Cargo rebuilds `rust_jsc_sys` when any supported or legacy variable changes.

## Common Workflows

### Use The Default Released Static Archive

```bash
cargo build
```

This downloads and links the target archive from:

```text
https://github.com/kevincaicedo/rust-jsc/releases/download/sys-v<version>/libjsc-<target>.a.gz
```

To make the mode explicit:

```bash
RUST_JSC_BUILD_MODE=download cargo build
```

### Use A Local Static JSC Build

From the repository root:

```bash
make build-jsc-static
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib" \
cargo test --lib -- --test-threads=1
```

`make test-local-jsc` runs this local static path for the library tests.

### Build JSC From Source During Cargo Build

Use source mode when Cargo should configure and build JSC directly:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_JOBS=8 \
cargo build
```

Equivalent stable alias:

```bash
RUST_JSC_FROM_SOURCE=1 \
RUST_JSC_JOBS=8 \
cargo build
```

To use a custom checkout or build directory:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_WEBKIT_DIR="$PWD/WebKit" \
RUST_JSC_BUILD_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static" \
cargo build
```

To create a local archive from a source build:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_ARCHIVE_OUT="$PWD/libjsc-local.a.gz" \
cargo build
```

`RUST_JSC_ARCHIVE_OUT` packages the `.a` files found in the source build's
`lib` directory and writes `<archive>.sha256` plus `<archive>.metadata.json`.
It is useful for local validation, not a replacement for the release Docker
archive workflow.

### Use A Different Archive Source

Use `RUST_JSC_MIRROR` when release assets are mirrored internally:

```bash
RUST_JSC_MIRROR=https://example.com/rust-jsc-releases cargo build
```

The build script requests:

```text
https://example.com/rust-jsc-releases/sys-v<version>/libjsc-<target>.a.gz
```

Use `RUST_JSC_ARCHIVE` for one exact archive:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_ARCHIVE=/path/to/libjsc-x86_64-unknown-linux-gnu.a.gz \
cargo build
```

`RUST_JSC_ARCHIVE` also accepts `file://`, `http://`, and `https://` URLs.
Local exact archives need `<archive>.sha256` or a `SHA256SUMS` file in the same
directory. HTTP exact archives use `<archive-url>.sha256`.

### Run ASAN And UBSAN Validation

Sanitizer builds are validation-only builds. They do not replace the default
static GitHub mirror archives and they are not published by the release archive
workflow.

From the repository root:

```bash
make test-webkit-api-asan
make test-webkit-api-ubsan
make test-rust-asan
make test-rust-ubsan
```

The WebKit API targets configure local JSCOnly trees with
`-DENABLE_SANITIZERS=address` or `-DENABLE_SANITIZERS=undefined`, build `jsc`,
`TestWTF`, and `TestJavaScriptCore`, then run the API test binaries directly.

Rust ASAN uses nightly Rust with `-Zsanitizer=address`, `-Zbuild-std`, and the
ASAN-instrumented static JSC build. Rust UBSAN links tests against the
UBSAN-instrumented JSC build with `-C link-arg=-fsanitize=undefined` and enables
Rust `-Zub-checks=yes` where nightly supports it.

### Build Release Linux Archives With Docker

The Dockerfiles produce release archive contents and copy `.a` files into the
Docker output directory.

```bash
make build-docker-jsc
python3 scripts/package_jsc_archive.py \
  --lib-dir .libs \
  --target-triple x86_64-unknown-linux-gnu \
  --output-dir "$PWD" \
  --repo-root "$PWD" \
  --webkit-dir WebKit
```

For musl:

```bash
make build-docker-jsc-musl
python3 scripts/package_jsc_archive.py \
  --lib-dir .libs-musl \
  --target-triple x86_64-unknown-linux-musl \
  --output-dir "$PWD" \
  --repo-root "$PWD" \
  --webkit-dir WebKit
```

For aarch64 cross builds:

```bash
make build-docker-jsc-arm
python3 scripts/package_jsc_archive.py \
  --lib-dir .libs-arm \
  --target-triple aarch64-unknown-linux-gnu \
  --output-dir "$PWD" \
  --repo-root "$PWD" \
  --webkit-dir WebKit
```

The GitHub `build-release.yml` workflow builds the platform matrix and publishes
the archives, checksum sidecars, metadata JSON, and combined `SHA256SUMS` under
a `sys-v<rust_jsc_sys version>` release tag. Docker layer caches are keyed by
the Dockerfile, Makefile, and sys crate manifest, with broad restore keys so
incremental release rebuilds can reuse stable layers.

### Use A System JavaScriptCore

```bash
RUST_JSC_BUILD_MODE=system \
RUST_JSC_SYSTEM_LIBS_PATH=/usr/local/lib \
RUST_JSC_SYSTEM_LIB_NAME=JavaScriptCore \
cargo build
```

This is for experiments only. Stock system JavaScriptCore usually cannot link
all rust-jsc APIs.

## CI And Release Behavior

- Pull-request CI checks whether the current `rust_jsc_sys` version already has
  a released Linux glibc x86_64 static archive and `SHA256SUMS` manifest.
- If the release exists, CI uses default `download` mode and lets
  `sys/build.rs` download and verify it from the GitHub mirror.
- If the release does not exist, CI builds the static JSC archive with Docker
  and sets `RUST_JSC_BUILD_MODE=download` plus `RUST_JSC_LIB_DIR=.libs`.
- The release workflow builds static archives for macOS, Linux glibc, and Linux
  musl targets, packages deterministic archives, verifies checksums, then
  publishes them to a `sys-v<version>` GitHub release.
- The crate publish workflow tests with `RUST_JSC_BUILD_MODE=download`, which
  downloads the published static archive when no local library directory is set.
- The sanitizer workflow is manual and scheduled. It builds local ASAN/UBSAN
  JSCOnly trees and runs WebKit API tests plus Rust integration tests, but it
  does not publish or alter release archives.

## Troubleshooting

### The Build Tries To Download An Archive That Does Not Exist

Confirm that `sys/Cargo.toml` version has a matching GitHub release tag and
`SHA256SUMS` file:

```text
sys-v<version>
SHA256SUMS
```

For local work before publishing a new sys archive, build JSC locally and set:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib \
cargo build
```

### Source Mode Cannot Configure CMake

Source mode requires CMake and Ninja:

```bash
cmake --version
ninja --version
```

Install `ninja-build` on Debian/Ubuntu or the equivalent package on your
distribution.

### A Local Static Linux Build Cannot Find `libstdc++.a` Or ICU `.a` Files

This is expected for many source-built local glibc builds. The build script
falls back to dynamic Linux system libraries when the static dependency archives
are not bundled beside the local JSC archives. Install the development packages
for your distro, for example:

```bash
sudo apt-get install libicu-dev libstdc++-dev libatomic1
```

Release Docker archives still bundle the static dependency archives and keep the
default downloaded archive path static.

### Runtime Loader Cannot Find A Dynamic System Build

When using `system` mode with dynamic libraries, set the platform loader path if
needed:

```bash
# macOS
export DYLD_LIBRARY_PATH=/path/to/jsc/lib:$DYLD_LIBRARY_PATH

# Linux
export LD_LIBRARY_PATH=/path/to/jsc/lib:$LD_LIBRARY_PATH
```

Static archive builds do not need these variables for JavaScriptCore itself.

### CMake Reuses The Wrong Build Directory

If a CMake cache was created from a different checkout path, use a fresh build
directory:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_BUILD_DIR="$PWD/WebKit/WebKitBuild/RustJSC-local/JSCOnly/Release-Static" \
cargo build
```
