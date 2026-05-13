# rust-jsc-sys

`rust_jsc_sys` provides the raw FFI bindings and build script used by
`rust_jsc`. It links against the JavaScriptCore artifacts produced from the
Kedo WebKit fork, not a stock system JavaScriptCore, because rust-jsc depends on
fork-only APIs for modules, synthetic modules, inspector integration, shared
data, typed arrays, and error helpers.

For normal users, no configuration is required. The default build downloads a
prebuilt static JavaScriptCore archive from the rust-jsc GitHub release mirror
and links it into the Rust crate.

## Default Behavior

With no environment variables set, `sys/build.rs` uses:

```text
RUST_JSC_BUILD_MODE=auto
```

`auto` behaves as follows:

1. If `RUST_JSC_CUSTOM_BUILD_PATH` is set, inspect that path and link the first
   supported artifact layout found there: framework, static archive set, or
   dynamic libraries.
2. Otherwise, download the prebuilt static archive for the current target from:

```text
https://github.com/kevincaicedo/rust-jsc/releases/download/sys-v<rust_jsc_sys version>/libjsc-<target>.a.gz
```

The downloaded archive is extracted into Cargo's build output directory and is
reused on later builds for the same version.

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

All archives include the JavaScriptCore static libraries:

```text
libJavaScriptCore.a
libWTF.a
libbmalloc.a
```

Some WebKit/JSCOnly builds also produce a separate JIT archive:

```text
libJavaScriptCoreJIT.a
```

When present, `rust_jsc_sys` links it after `libJavaScriptCore.a`. When it is
not present, `libJavaScriptCore.a` is expected to be self-contained for that
target.

Linux release archives also include the static system dependency archives used
by the release Docker builds:

```text
libstdc++.a
libicui18n.a
libicuuc.a
libicudata.a
libatomic.a
```

The CI release workflow verifies these files for Linux archives before
publishing. This is separate from local Linux development builds, which may link
system C++/ICU/atomic libraries dynamically when those static dependency
archives are not present beside the local JSC archives.

## Build Modes

Select a mode with `RUST_JSC_BUILD_MODE`.

| Mode | Use When | Behavior |
| --- | --- | --- |
| `auto` | Default for users and CI that should use released archives. | Uses `RUST_JSC_CUSTOM_BUILD_PATH` if set, otherwise downloads the target static archive from the GitHub mirror. |
| `download` | You want to force archive download and ignore local builds. | Downloads and links the prebuilt static archive. Alias: `archive`. |
| `static` | You have local static JSC archives, or you want static archive download fallback. | Uses `RUST_JSC_CUSTOM_BUILD_PATH` when set; otherwise downloads the prebuilt static archive. |
| `framework` | You have a macOS `JavaScriptCore.framework` build. | Links the framework from `RUST_JSC_FRAMEWORK_PATH` or `RUST_JSC_CUSTOM_BUILD_PATH`. |
| `source` | You want Cargo to configure/build JavaScriptCore from the bundled WebKit checkout. | Runs CMake/JSCOnly, then auto-detects artifacts in the build directory. |
| `system` | You are experimenting with a system-provided JavaScriptCore. | Links a system dylib/framework. This may not provide rust-jsc fork-only APIs. |

`RUST_JSC_FROM_SOURCE=1` takes precedence over `RUST_JSC_BUILD_MODE` and selects
`source`.

`RUST_JSC_FRAMEWORK_PATH` takes precedence over the mode string and selects
`framework`.

## Configuration Variables

| Variable | Applies To | Description |
| --- | --- | --- |
| `RUST_JSC_BUILD_MODE` | All builds | Selects `auto`, `download`, `static`, `framework`, `source`, or `system`. Defaults to `auto`. |
| `RUST_JSC_CUSTOM_BUILD_PATH` | `auto`, `static`, `framework` | Directory containing JSC artifacts. May point directly at a `lib` directory, a CMake build directory with `lib`, `JavaScriptCore.framework`, or the framework parent directory. |
| `RUST_JSC_CUSTOM_ARCHIVE` | `download`, `static` fallback, `auto` fallback | Overrides the archive URL/path used for download mode. |
| `RUST_JSC_MIRROR` | `download`, `static` fallback, `auto` fallback | Overrides the default release mirror base URL. The build script appends `/sys-v<version>/libjsc-<target>.a.gz`. |
| `RUST_JSC_FRAMEWORK_PATH` | `framework` | Path to `JavaScriptCore.framework` or its parent directory. |
| `RUST_JSC_FROM_SOURCE` | All builds | Truthy values (`1`, `true`, `yes`, `on`) force `source` mode. |
| `RUST_JSC_WEBKIT_DIR` | `source` | WebKit checkout to build. Defaults to `../WebKit` relative to the `sys` crate. |
| `RUST_JSC_BUILD_DIR` | `source` | CMake build directory. Defaults to `WebKit/WebKitBuild/RustJSC/JSCOnly/<profile>`. |
| `RUST_JSC_BUILD_PROFILE` | `source` | CMake build type. Defaults to `Release`. |
| `RUST_JSC_CMAKE_GENERATOR` | `source` | CMake generator, for example `Ninja`. If unset, the build script uses Ninja when available. |
| `RUST_JSC_JOBS` | `source` | Parallel build job count. Defaults to the host parallelism or `4`. |
| `RUST_JSC_STATIC` | `source` | Truthy values add `-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF` to the CMake configure. |
| `RUST_JSC_FORCE_CMAKE_CONFIGURE` | `source` | Truthy values rerun CMake configure even if `CMakeCache.txt` exists. |
| `RUST_JSC_FORCE_SOURCE_BUILD` | `source` | Truthy values rebuild JSC even when artifacts already exist. |
| `RUST_JSC_SYSTEM_LIBS_PATH` | `system` | Extra native library search directory for system JavaScriptCore. |
| `RUST_JSC_SYSTEM_LIB_NAME` | `system` | Dynamic library name to link in system mode. Defaults to `JavaScriptCore`. |
| `SYSTEM_LIBS_PATH` | macOS static | Extra search path for macOS system libraries. Defaults to `/usr/lib`. |

Cargo rebuilds `rust_jsc_sys` when any of these variables changes.

## Common Workflows

### Use The Default Released Static Archive

This is the normal user path:

```bash
cargo build
```

The build script downloads the archive matching the target triple and links it.

To make the static download explicit:

```bash
RUST_JSC_BUILD_MODE=static cargo build
```

### Use A Different Archive Mirror

Use `RUST_JSC_MIRROR` when release assets are mirrored internally:

```bash
RUST_JSC_MIRROR=https://example.com/rust-jsc-releases cargo build
```

The build script will request:

```text
https://example.com/rust-jsc-releases/sys-v<version>/libjsc-<target>.a.gz
```

Use `RUST_JSC_CUSTOM_ARCHIVE` when you want a specific archive file or URL:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_CUSTOM_ARCHIVE=/path/to/libjsc-x86_64-unknown-linux-gnu.a.gz \
cargo build
```

### Build Static JSC Locally For Development

From the repository root:

```bash
make build-jsc-static
RUST_JSC_BUILD_MODE=static \
RUST_JSC_CUSTOM_BUILD_PATH="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib" \
cargo test --lib -- --test-threads=1
```

`make build-jsc` is also static by default because `JSC_STATIC=ON` in the
Makefile. Use `make build-jsc-framework` when you explicitly want a macOS
framework/dynamic build.

On local Linux glibc builds, the static JSC archives normally sit beside no
static `libstdc++` or ICU archives. In that case `rust_jsc_sys` links JSC/WTF
statically but links `stdc++`, ICU, and `atomic` dynamically from the host. This
is intended for local development. Release Docker archives still bundle those
static dependency archives.

### Build Static JSC During Cargo Build

Use this when the Cargo build itself should drive CMake:

```bash
RUST_JSC_FROM_SOURCE=1 \
RUST_JSC_STATIC=1 \
RUST_JSC_JOBS=8 \
cargo build
```

Set `RUST_JSC_WEBKIT_DIR` and `RUST_JSC_BUILD_DIR` to control source and build
locations:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_STATIC=1 \
RUST_JSC_WEBKIT_DIR="$PWD/WebKit" \
RUST_JSC_BUILD_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static" \
cargo build
```

### Build Release Linux Archives With Docker

The Dockerfiles produce archive contents for release and CI. They copy `.a`
files into the Docker output directory.

```bash
make build-docker-jsc
cd .libs
tar -czf ../libjsc-x86_64-unknown-linux-gnu.a.gz *.a
```

For musl:

```bash
make build-docker-jsc-musl
cd .libs-musl
tar -czf ../libjsc-x86_64-unknown-linux-musl.a.gz *.a
```

For aarch64 cross builds:

```bash
make build-docker-jsc-arm
cd .libs-arm
tar -czf ../libjsc-aarch64-unknown-linux-gnu.a.gz *.a
```

The GitHub `build-release.yml` workflow builds the complete platform matrix and
publishes those archives under a `sys-v<rust_jsc_sys version>` release tag.

### Use A macOS Framework Build

```bash
make build-jsc-framework
RUST_JSC_BUILD_MODE=framework \
RUST_JSC_CUSTOM_BUILD_PATH="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release/lib" \
cargo test
```

Use framework mode for local macOS debugging or symbol inspection. It is not the
default release path.

### Use A System JavaScriptCore

```bash
RUST_JSC_BUILD_MODE=system \
RUST_JSC_SYSTEM_LIBS_PATH=/usr/local/lib \
RUST_JSC_SYSTEM_LIB_NAME=JavaScriptCore \
cargo build
```

System mode is for experiments only. Distribution-provided JavaScriptCore builds
usually do not export rust-jsc's custom APIs, so normal rust-jsc features may
fail to link.

## CI And Release Behavior

- Pull-request CI checks whether the current `rust_jsc_sys` version already has
  a released Linux glibc x86_64 static archive.
- If the release exists, CI lets `sys/build.rs` download it from the GitHub
  mirror.
- If the release does not exist, CI builds the static JSC archive with Docker
  and sets `RUST_JSC_BUILD_MODE=static` plus `RUST_JSC_CUSTOM_BUILD_PATH=.libs`.
- The release workflow always builds static archives for macOS, Linux glibc, and
  Linux musl targets, then publishes them to a `sys-v<version>` GitHub release.
- The crate publish workflow tests with `RUST_JSC_BUILD_MODE=static`, which
  downloads the published static archive when no custom path is set.

## Troubleshooting

### The Build Tries To Download An Archive That Does Not Exist

Confirm that `sys/Cargo.toml` version has a matching GitHub release tag:

```text
sys-v<version>
```

For local work before publishing a new sys archive, build JSC locally and set:

```bash
RUST_JSC_BUILD_MODE=static \
RUST_JSC_CUSTOM_BUILD_PATH=/path/to/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib \
cargo build
```

### A Local Static Linux Build Cannot Find `libstdc++.a` Or ICU `.a` Files

This is expected for many source-built local glibc builds. The build script now
falls back to dynamic Linux system libraries when the static dependency archives
are not bundled beside the local JSC archives. Install the development packages
for your distro, for example:

```bash
sudo apt-get install libicu-dev libstdc++-dev libatomic1
```

Release Docker archives still bundle the static dependency archives and keep the
default downloaded archive path static.

### Runtime Loader Cannot Find A Dynamic Local JSC Build

When using `framework`, `system`, or dynamic library layouts, set the platform
loader path if needed:

```bash
# macOS
export DYLD_LIBRARY_PATH=/path/to/jsc/lib:$DYLD_LIBRARY_PATH

# Linux
export LD_LIBRARY_PATH=/path/to/jsc/lib:$LD_LIBRARY_PATH
```

Static archive builds do not need these variables for JavaScriptCore itself.

### CMake Reuses The Wrong Build Directory

If a CMake cache was created from a different checkout path, use a fresh build
directory instead of deleting unrelated user artifacts:

```bash
make build-jsc JSC_STATIC=ON JSC_BUILD_ROOT=WebKit/WebKitBuild/RustJSC-local JSC_JOBS=8
```
