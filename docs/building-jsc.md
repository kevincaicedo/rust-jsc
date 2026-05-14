# Building JavaScriptCore For rust-jsc

rust-jsc links against the Kedo WebKit fork, not a stock JavaScriptCore build.
The fork exposes APIs used by the safe module loader, synthetic modules,
inspector sessions, shared data, typed arrays, and error helpers.

The default path is the released static archive:

```bash
cargo build
```

With no environment variables, `rust_jsc_sys` uses
`RUST_JSC_BUILD_MODE=download`, downloads `libjsc-<target>.a.gz` from the
`sys-v<rust_jsc_sys version>` GitHub release, verifies the exact matching
`SHA256SUMS` entry, rejects unsafe archive paths, extracts the archive into
Cargo's build output directory, and links it statically.

## Choose A Build Mode

| Mode | Use when | Behavior |
| --- | --- | --- |
| `download` | Normal development, CI, release validation, or a local prebuilt static JSC. | Uses `RUST_JSC_ARCHIVE` first, `RUST_JSC_LIB_DIR` second, then the release mirror. |
| `source` | Cargo should configure and build the bundled WebKit checkout. | Runs direct CMake/JSCOnly with Ninja and static JSC enabled. |
| `system` | You are investigating a system or dynamic JavaScriptCore. | Links a system library or framework. This is experimental because stock JSC usually lacks rust-jsc fork APIs. |

Legacy mode names remain accepted during the 1.0 migration window, but new
scripts should use the three names above. The exhaustive environment variable
reference lives in the
[`rust_jsc_sys` build configuration reference](https://github.com/kevincaicedo/rust-jsc/blob/main/sys/README.md).

## Use A Released Archive

This is the default user path:

```bash
RUST_JSC_BUILD_MODE=download cargo test --lib -- --test-threads=1
```

The release URL is:

```text
https://github.com/kevincaicedo/rust-jsc/releases/download/sys-v<version>/libjsc-<target>.a.gz
```

Every published release must include a combined `SHA256SUMS` manifest, per
archive `.sha256` sidecars, and metadata JSON files. The build script verifies
the selected archive before extraction. Release workflows also check the full
supported target set before crate release creation or publishing, so a partial
`sys-v<version>` archive release is treated as a release gap.

## Use A Local Static Build

Use this when validating WebKit fork changes or working before a new
`rust_jsc_sys` archive release exists:

```bash
make build-jsc-static
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib" \
cargo test --lib -- --test-threads=1
```

`make test-local-jsc` runs the same local-static test path. On Linux, local
CMake static builds may not include static `stdc++`, ICU, or `atomic` archives
beside JSC. In that case the build script links JSC/WTF statically and falls
back to dynamic host system libraries. Release Docker archives keep those
dependencies bundled.

## Build From Source Through Cargo

Use source mode when Cargo should build the bundled WebKit checkout:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_JOBS=8 \
cargo build
```

To select a custom WebKit checkout or build directory:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_WEBKIT_DIR="$PWD/WebKit" \
RUST_JSC_BUILD_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static" \
cargo build
```

Source mode always uses direct CMake/JSCOnly, Ninja, and static archives. It is
the Cargo-supported source path; `WebKit/Tools/Scripts/build-jsc` remains only a
manual fallback for WebKit investigations.

## Package A Local Archive

For a local static JSC build:

```bash
make archive platform=x86_64-unknown-linux-gnu
```

For Docker-produced Linux release contents:

```bash
make build-docker-jsc
python3 scripts/package_jsc_archive.py \
  --lib-dir .libs \
  --target-triple x86_64-unknown-linux-gnu \
  --output-dir "$PWD" \
  --repo-root "$PWD" \
  --webkit-dir WebKit
```

The packager writes deterministic `.tar.gz` bytes, an archive `.sha256`
sidecar, and metadata JSON containing the archive hash, included library hashes,
target triple, WebKit commit, rust-jsc sys version, repository commit, and tool
versions.

## Troubleshooting

If the build tries to download an archive that does not exist, confirm that
`sys/Cargo.toml` has a matching GitHub release tag:

```text
sys-v<version>
```

For local work before that release exists, build JSC locally and set
`RUST_JSC_LIB_DIR`.

If source mode cannot configure CMake, check that CMake and Ninja are installed:

```bash
cmake --version
ninja --version
```

If CMake reuses a cache from another checkout, use a fresh build directory:

```bash
RUST_JSC_BUILD_MODE=source \
RUST_JSC_BUILD_DIR="$PWD/WebKit/WebKitBuild/RustJSC-local/JSCOnly/Release-Static" \
cargo build
```

If `system` mode links but runtime symbols are missing, switch back to
`download` or `source`. System JavaScriptCore builds usually do not export the
Kedo fork APIs required by rust-jsc.
