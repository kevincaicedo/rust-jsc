# Rust-JSC

[![Crates.io](https://img.shields.io/crates/v/rust_jsc.svg)](https://crates.io/crates/rust_jsc)
[![Docs.rs](https://docs.rs/rust_jsc/badge.svg)](https://docs.rs/rust_jsc)

Rust-JSC is a Rust-native JavaScriptCore binding focused on safe embedding,
explicit ownership, predictable and a small reusable API surface.

## Features

- Owned and borrowed context handles with explicit JavaScriptCore lifetime rules
- Safe value, object, array, typed-array, function, promise, module, and error wrappers
- ES module loading through Rust-owned resolver, fetcher, import-meta, JSON, Wasm, and synthetic-module policy
- Typed callback, constructor, and module-loader macros backed by conversion traits and contained FFI panic boundaries
- Static JavaScriptCore linking.
- Direct inspector channel primitives for debugger and runtime integrations

## Documentation

- [API design guide](docs/api-design.md): ownership vocabulary, conversion traits, builders, macros, unsafe escape hatches, and performance rules.
- [Macro guide](docs/macro-guide.md): supported signatures, typed callback/constructor conversions, class lifecycle macros, module-loader macros, and inspector callbacks.
- [Module guide](docs/modules.md): file/custom loaders, JSON modules, WebAssembly modules, synthetic modules, and host pump behavior.
- [Object property guide](docs/object-properties.md): property keys, symbol keys, descriptors, property-name iteration, and exception behavior.
- [Typed arrays](docs/typed-arrays.md): safe copied reads, owned buffer transfer, ArrayBuffer construction, and unsafe borrowed views.
- [Embedding guide](docs/embedding.md): minimal runtime shape, callbacks, module pump checkpoints, promises, and inspector sessions.
- [Inspector guide](docs/inspector.md): direct inspector sessions, pause-loop pump rules, and CDP bridge expectations.
- [Memory management](docs/memory_management.md): private/shared data ownership and finalization.
- [Class guide](docs/jsclass.md): class builder and custom-object private data.
- [Build guide](docs/building-jsc.md): build-mode selection, released archives, local static builds, source builds, packaging, and troubleshooting.
- [Release process](docs/release-process.md): archive matrix, checksum and metadata gates, crate publishing order, validation, and rollback.
- [Safety model](docs/safety-model.md): context affinity, exception propagation, private/shared data, callbacks, typed arrays, inspector sessions, and unsafe contracts.
- [Performance validation](docs/performance-validation.md): benchmark workloads, snapshot artifacts, CI guardrails, and deferred budget rules.
- [Build configuration](https://github.com/kevincaicedo/rust-jsc/blob/main/sys/README.md): exhaustive `rust_jsc_sys` environment variable and raw linking reference.

## Examples

Focused runnable examples live in `examples/api_showcase`:

```bash
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin classes
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin callbacks
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin modules_file
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin modules_custom_loader
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin typed_arrays
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin promises
cargo run --manifest-path examples/api_showcase/Cargo.toml --bin kedo_integration
```

The existing `examples/modules`, `examples/debugger`, `examples/profiling`, and
`examples/stress` crates cover broader module, inspector, heap-profiling, and
stress scenarios.

## Installation

Add the following line to your `Cargo.toml` file:

```toml
[dependencies]
rust_jsc = { version = "1.0.0" }
```

## Usage

### Context And Group Ownership

`JSContext::new()` returns an owned `JSGlobalContext` (`OwnedJSContext` is an alias). Owned global contexts release their `JSGlobalContextRef` in `Drop`, so manual `release()` is only needed when you want to consume the handle early.

`JSContext` is a borrowed view. JavaScriptCore callbacks receive `JSContext` because the callback does not own the context and must not release it. If you need to keep a callback context beyond the callback owner, call `ctx.retain()` and store the returned `JSGlobalContext`.

`JSContextGroup` is also borrowed. `JSContextGroup::new()` returns an `OwnedJSContextGroup`, and `ctx.group()` returns a borrowed group view. Call `group.retain()` when Rust needs a retained group handle.

Context and group handles are intentionally not `Send` or `Sync`. Use a context and its values on the thread that owns the JavaScriptCore VM/group unless you build a higher-level synchronization layer around raw JavaScriptCore usage.

### Evaluate Script

```rust
use rust_jsc::{JSContext, JSResult};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let result = ctx.evaluate_script("console.log('Hello, world!'); 'kedojs'", Some(0))?;
    assert_eq!(result.as_string()?.to_string(), "kedojs");
    Ok(())
}
```

### Evaluate Module

```rust
use rust_jsc::{JSContext, JSResult, ModuleLoader};

fn main() -> JSResult<()> {
    let filename = "/path/filename.js";
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let promise = ctx.evaluate_module(filename)?;
    assert!(promise.is_object());
    ctx.run_deferred_work(); // Needed when WebAssembly module compilation is pending.
    ctx.run_microtasks();
    Ok(())
}
```

`ModuleLoader::file_system()` returns rust-jsc's default file module loader.
The loader lives in Rust and supports absolute paths, `file://` URLs,
`./`/`../` specifiers relative to the importing module, bare `.json` imports,
JSON import attributes, `.wasm` modules, dynamic import, and `import.meta.url`.
The Kedo-WebKit fork only receives resolve/fetch/import-meta callbacks.

Module APIs return JavaScript promises. They do not drain microtasks internally;
call `run_deferred_work()` and `run_microtasks()` only at explicit host
event-loop checkpoints. Custom resolver, fetcher, and import-meta callbacks can
return `ModuleLoadError` when diagnostics need a module id and optional
referrer without moving runtime-specific policy into `rust-jsc`.

See [docs/modules.md](docs/modules.md) for custom loaders, synthetic modules,
JSON modules, WebAssembly modules, dynamic import, and host-pump guidance.

### API Notes

Typed callbacks and constructors use `TryFromJSValue` for argument conversion.
Missing optional arguments, JavaScript `undefined`, and JavaScript `null` map to
`Option::None`. `Rest<T>` captures final-position rest arguments. Typed macro
arguments are parsed from JavaScriptCore's raw argument slice; the macro
allocates an argument `Vec` only for the explicit legacy `&[JSValue]` form.
Callback and constructor returns use `IntoJSResult`, so `JSResult<JSValue>`
still works while typed returns such as `String`, `bool`, `()`, `JSObject`, or
`JSResult<T>` where `T: IntoJSValue` are converted by the wrapper.
Ergonomic callback signatures may omit the old JavaScriptCore ABI prefix and
receive only JavaScript arguments, for example `fn add(left: f64, right: f64)`.
Use `CallbackContext`, `CallbackFunction`, `ThisObject`, and
`ConstructorObject` when a callback or constructor needs the borrowed context,
callee, receiver, or constructor object explicitly. The legacy
`(JSContext, JSObject, JSObject, ...)` callback and
`(JSContext, JSObject, ...)` constructor forms remain supported.
Direct function/object calls can also convert results through
`TryFromJSValue`: use `JSFunction::call_typed`,
`JSFunction::call_constructor_typed`, `JSObject::call_typed`,
`JSObject::call_as_constructor_typed`, or `JSObject::call_method_typed` when
the expected Rust return type is known.
`JSFunction::name`, `JSFunction::display_name`, and `JSFunction::source`
provide fallible function metadata helpers through normal JavaScript property
and `toString` semantics.
Class lifecycle macros validate exact `#[initialize]` and `#[finalize]`
signatures and catch Rust panics before returning across JavaScriptCore's C ABI.
Use `JSClass::try_builder` when a class name is not a static Rust literal.
`JSClassBuilder::method` installs prototype methods through JavaScriptCore's
static function table, sharing one function entry through the generated
prototype rather than copying methods onto each instance.
`JSClassBuilder::constructor_method` installs static methods on the registered
class object, so embedders can expose `ClassName.method()` without per-instance
properties.
`JSClassBuilder::accessor` installs known properties through JavaScriptCore's
static value table. `JSClassBuilder::typed_accessor` uses
`JSClassAccessor` plus `TryFromJSValue`/`IntoJSValue` for typed getter/setter
conversion without a runtime registry. `JSClass` owns the single reference
returned by `JSClassCreate` and releases it in `Drop`.
`JSClass::object_with_prototype` and `JSObject::set_prototype_checked` provide
fallible explicit prototype setup for manual prototype ownership.
Typed module-loader macros expose resolver keys as `String`, referrers as
`Option<String>`, import attributes as `ModuleImportType`, and fetch results as
`ModuleSource` or `JSModuleSource`. They target `ModuleLoaderBuilder` callback
slots while the legacy `module_*` macros remain available for raw C ABI hooks.

`JSArray::length()` and `JSArray::push()` use exact JavaScript Array C APIs and
return `usize` lengths. They intentionally reject proxies and array-like objects;
use `JSObject` property access when generic JavaScript semantics are required.

Use `PropertyKey` plus `JSObject::{set,get,has,delete}_property_by_key` when an
API accepts string names, JavaScript value keys, symbol keys, and array-index
keys. `PropertyKey::symbol(&ctx, "name")` creates a symbol-keyed property path.
These methods check value keys and values against the object's context before
crossing JavaScriptCore and return `JSError` when value-key conversion throws.
`PropertyDescriptor::builder()` has named shortcuts such as `read_only`,
`non_enumerable`, and `non_configurable`; use the boolean setters only when
translating runtime policy. Property-name iteration is RAII-managed and
implements exact-size fused iteration.

Use `JSClass::set_object_private_data` to attach private data after creating a
class object. It checks the class/type contract and refuses to replace occupied
private-data slots; raw replacement remains an unsafe escape hatch.
`get_private_data` and `get_shared_data` return shared RAII guards, while
`get_private_data_mut` and `get_shared_data_mut` return exclusive guards and
refuse aliasing. Safe take/drop/replace operations report `Borrowed` instead of
invalidating active guards.

Use `JSValue::protected()` to keep a JavaScript value alive across Rust-owned
storage. The returned `ProtectedValue` calls `JSValueProtect` on creation and
`JSValueUnprotect` in `Drop`.
Use `ProtectedObject::new(object)` or `JSObject::into_protected()` when Rust
needs to keep an object-typed handle callable or otherwise accessible across a
host callback table, future completion, or runtime resource slot.

Use `Promise::new_pending()` or `JSPromise::new_pending()` for deferred
promises. The returned `PromiseResolver` is RAII-protected: dropping the
resolver releases its resolve/reject function protections, and `Promise` keeps
its own resolver handle for `resolve`/`reject` convenience methods. Promise
handles remain JavaScriptCore-thread-affine.

Use `JSContext::set_unhandled_rejection_handler(&function)` to install a
callable same-context JavaScript handler. The returned
`UnhandledRejectionHandler` protects the callback while the host stores the
guard; dropping the guard releases Rust's protection count but does not
unregister JavaScriptCore's global handler.

Typed-array and ArrayBuffer constructors make ownership explicit.
`JSTypedArray::with_bytes` and `JSArrayBuffer::from_bytes` copy Rust slices into
JavaScriptCore-owned storage. `JSTypedArray::with_owned_bytes` and
`JSArrayBuffer::from_vec` transfer a Rust `Vec` to JavaScriptCore with a Rust
deallocator and avoid copying the vector buffer. Borrowed no-copy constructors
and borrowed byte-slice accessors are `unsafe` because JavaScriptCore byte
pointers are temporary and caller-owned buffers can otherwise outlive Rust
slices. See [docs/typed-arrays.md](docs/typed-arrays.md).

`JSObject::call_method(...)` is the safe wrapper for common method calls that
must preserve JavaScript property lookup and `this` binding. `JSPromise` and
`JSRegExp` use it internally for `then`/`catch`/`finally` and `exec`/`test`.
Calls, constructor calls, method calls, array construction, and error
construction avoid heap allocation for argument lists of up to eight values.
Date and RegExp construction use the same small-argument path.

Use `JSContext::inspector_session()` for a direct JavaScriptCore inspector
frontend. `InspectorSession` installs the message callback, optionally installs
a pause-loop callback, sends protocol messages through a fallible
`send_message` API, accepts validated `InspectorOutboundMessage` values through
`send_protocol_message`, and disconnects on drop. The lower-level
`JSContext::inspector_send_message` API is also fallible so protocol strings
with interior NUL bytes do not panic.

Inspector callbacks receive borrowed UTF-8 message slices from JavaScriptCore.
Wrap them in `InspectorInboundMessage` to name the callback lifetime and copy
them into `OwnedInspectorMessage` before queueing or crossing async/thread
boundaries. Pause-loop callbacks are low-level pump notifications; do not
release the context, destroy the VM, block indefinitely, or run arbitrary
JavaScript from them.

### Typed Arrays

```rust
use rust_jsc::{JSArrayBuffer, JSContext, JSResult, JSTypedArray, JSTypedArrayType};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let array = ctx
        .evaluate_script("const array = new Uint8Array([5, 4, 4, 5]); array", None)?;
    let array = JSTypedArray::from_value(&array)?;

    assert_eq!(array.array_type()?, JSTypedArrayType::Uint8Array);
    assert_eq!(array.len()?, 4);
    assert_eq!(array.byte_offset()?, 0);
    assert_eq!(array.byte_len()?, 4);
    assert_eq!(array.as_vec::<u8>()?, &[5, 4, 4, 5]);

    let buffer = JSArrayBuffer::from_vec(&ctx, vec![1, 2, 3, 4])?;
    let owned = JSTypedArray::with_buffer(&ctx, buffer, JSTypedArrayType::Uint8Array)?;
    assert_eq!(owned.as_vec::<u8>()?, vec![1, 2, 3, 4]);
    Ok(())
}
```

### Array

```rust
use rust_jsc::{JSArray, JSContext, JSResult, JSValue};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let array = JSArray::new_array(
        &ctx,
        &[
            JSValue::number(&ctx, 1.0),
            JSValue::number(&ctx, 2.0),
            JSValue::number(&ctx, 3.0),
        ],
    )?;
    assert_eq!(array.as_string()?, "1,2,3");
    Ok(())
}
```

### Callbacks

```rust
use rust_jsc::{callback, JSContext, JSFunction, JSObject, JSResult, JSValue};

#[callback]
fn log_info(message: String) {
    println!("INFO: {}", message);
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let console = JSObject::new(&ctx);
    let log = JSFunction::callback(&ctx, Some("log"), Some(log_info));
    let log_value: JSValue = log.into();

    console.set_property("log", &log_value, Default::default())?;
    let console_value: JSValue = console.into();
    ctx.global_object()
        .set_property("console", &console_value, Default::default())?;

    ctx.evaluate_script("console.log('Hello, World!')", None)?;
    Ok(())
}
```

### Synthetic Modules

```rust
use rust_jsc::{
    module_import_meta_provider, module_resolver, JSContext, JSObject, JSResult,
    JSValue, ModuleLoadError, ModuleLoaderBuilder,
};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> Result<Option<String>, ModuleLoadError> {
    Ok(Some(specifier))
}

#[module_import_meta_provider]
fn import_meta(
    ctx: JSContext,
    key: String,
) -> JSResult<Option<JSObject>> {
    let meta = JSObject::new(&ctx);
    meta.set_property(
        "url",
        &JSValue::string(&ctx, format!("runtime://{key}")),
        Default::default(),
    )?;
    Ok(Some(meta))
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    ctx.set_module_loader(
        ModuleLoaderBuilder::new()
            .resolve(Some(resolve))
            .import_meta(Some(import_meta)),
    );

    let name = JSValue::string(&ctx, "John Doe");
    ctx.create_synthetic_module("@rust-jsc", &[("default", &name), ("name", &name)])?;

    ctx.evaluate_module_from_source(
        "import lib, { name } from '@rust-jsc';
         globalThis.name = name;
         globalThis.metaUrl = import.meta.url;",
        "example.js",
        None,
    )?;
    ctx.run_deferred_work();
    ctx.run_microtasks();

    assert_eq!(
        ctx.evaluate_script("globalThis.name", None)?
            .as_string()?
            .to_string(),
        "John Doe"
    );
    Ok(())
}
```

Synthetic modules are explicit. Create them with
`JSContext::create_synthetic_module()` before importing the resolved key.
Duplicate export names and cross-context export values are rejected, and
module loading/evaluation APIs return the JavaScript `Promise` created by
JavaScriptCore. The full module-loader guide is
[docs/modules.md](docs/modules.md).

## Supported Platforms

| Platform | Arch | Target | Status |
|----------|------|--------|--------|
| macOS    | x86_64 | x86_64-apple-darwin | ✅ |
| macOS    | aarch64 | aarch64-apple-darwin | ✅ |
| Linux    | x86_64 | x86_64-unknown-linux-gnu | ✅ |
| Linux    | aarch64 | aarch64-unknown-linux-gnu | ✅ |
| Linux    | x86_64 | x86_64-unknown-linux-musl | ✅ |
| Linux    | aarch64 | aarch64-unknown-linux-musl | ✅ |
| Windows  | x86_64 | x86_64-pc-windows-msvc | Not supported |

## JavaScriptCore Builds

`rust_jsc` uses the `rust_jsc_sys` build script to locate and link the Kedo
WebKit JavaScriptCore build. The default path is static: with no configuration,
the build script downloads a prebuilt `libjsc-<target>.a.gz` archive from the
rust-jsc GitHub release mirror, verifies the exact matching `SHA256SUMS` entry,
rejects unsafe archive paths, and links `libJavaScriptCore.a`, `libWTF.a`, and
`libbmalloc.a`; it also links `libJavaScriptCoreJIT.a` when a target produces a
separate JIT archive.

For local development against the bundled WebKit checkout:

```bash
make build-jsc-static
RUST_JSC_BUILD_MODE=download \
  RUST_JSC_LIB_DIR="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib" \
  cargo test --lib -- --test-threads=1
```

Sanitizer validation is separate from the release archive path:

```bash
make test-webkit-api-asan
make test-webkit-api-ubsan
make test-rust-asan
make test-rust-ubsan
```

These targets build local ASAN/UBSAN JSCOnly trees and run WebKit
`TestWTF`/`TestJavaScriptCore` plus Rust tests where practical. Rust ASAN uses
nightly `-Zsanitizer=address`; Rust UBSAN links against UBSAN-instrumented
WebKit and enables Rust UB checks because Rust does not expose
`-Zsanitizer=undefined`.

Linux release archives are built in Docker with deterministic packaging,
per-archive `.sha256` sidecars, metadata JSON, and a combined `SHA256SUMS`.
The release workflows fail if any supported archive, sidecar, metadata file, or
exact manifest row is missing. Linux archives include bundled static
`libstdc++`, ICU, and `libatomic` archives. Local Linux source builds may link
those system dependencies dynamically when the static `.a` files are not
present beside the local JSC archives; that is intended for development and
does not change the default release archive path.

For every build mode and environment variable, see the
[`rust_jsc_sys` build configuration reference](https://github.com/kevincaicedo/rust-jsc/blob/main/sys/README.md).

## FAQ

### How do I build JavaScriptCore locally?

Use the direct CMake/JSCOnly Makefile target. The default local build is static.

```bash
make build-jsc-static
make jsc-smoke
make test-local-jsc
```

Linux release archives are produced with Docker:

```bash
make build-docker-jsc
make build-docker-jsc-musl
```

The complete build configuration reference lives in the
[`rust_jsc_sys` build configuration reference](https://github.com/kevincaicedo/rust-jsc/blob/main/sys/README.md).

> :warning: This crate uses a custom [Kedo WebKit](https://github.com/kevincaicedo/Kedo-WebKit)
> fork. Stock system JavaScriptCore builds usually do not export the APIs that
> rust-jsc needs.
>
> The crate-facing fork contract is documented in the
> [build guide](docs/building-jsc.md) and
> [release process](docs/release-process.md): patch-noise stays out of
> releases, fork APIs are tested through CI, and filesystem/module policy stays
> in Rust-owned loaders rather than WebKit internals.

### How do I troubleshoot linking problems?

For static local builds, first confirm `RUST_JSC_LIB_DIR` points at
the directory containing `libJavaScriptCore.a`, `libWTF.a`, and `libbmalloc.a`.
For `system` mode experiments with dynamic libraries, set the platform loader
path if needed:

```bash
# macOS
export DYLD_LIBRARY_PATH=/path/to/jsc/lib:$DYLD_LIBRARY_PATH
```

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/jsc/lib:$LD_LIBRARY_PATH
```

More troubleshooting notes are in the
[`rust_jsc_sys` build configuration reference](https://github.com/kevincaicedo/rust-jsc/blob/main/sys/README.md).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
