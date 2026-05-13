# Rust-JSC

[![Crates.io](https://img.shields.io/crates/v/rust-jsc.svg)](https://crates.io/crates/rust-jsc)
[![Docs.rs](https://docs.rs/rust_jsc/badge.svg)](https://docs.rs/rust_jsc)

Rust-JSC is a Rust library that provides a High-level binding for the JavaScriptCore engine. It allows you to interact with JavaScript code from your Rust applications.

## Features

- High-level binding to the JavaScriptCore engine
- Extended API to interact with JavaScriptCore
- Support for ES Modules
- Support for rust native modules (Synthetic Modules)


## Installation

Add the following line to your `Cargo.toml` file:

```toml
[dependencies]
rust_jsc = { version = "1.0.0" }
```

## Usage

### Evaluate Script

```rust
use rust_jsc::JSContext;

let ctx = JSContext::new();
let result = ctx.evaluate_script("console.log('Hello, world!'); 'kedojs'", Some(0));
assert!(result.is_ok());
```

### Evaluate Module

```rust
use rust_jsc::JSContext;

let filename = "/path/filename.js";
let ctx = JSContext::new();
let result = ctx.evaluate_module(filename);
assert!(result.is_ok());
```

### Typed Arrays

```rust
use rust_jsc::{JSArrayBuffer, JSContext, JSTypedArray, JSTypedArrayType};

fn main() {
    let ctx = JSContext::new();
    let array = ctx
        .evaluate_script("const array = new Uint8Array([5, 4, 4, 5]); array", None)
        .unwrap();
    let array = JSTypedArray::from_value(array).unwrap();

    assert_eq!(array.array_type().unwrap(), JSTypedArrayType::Uint8Array);
    assert_eq!(array.len().unwrap(), 4);
    assert_eq!(array.byte_offset().unwrap(), 0);
    assert_eq!(array.byte_len().unwrap(), 4);
    assert_eq!(array.as_vec::<u8>().unwrap(), &[5, 4, 4, 5]);
}
```

### Array

```rust
use rust_jsc::{JSArray, JSContext, JSValue};

let ctx = JSContext::new();
let array = JSArray::new_array(
    &ctx,
    &[
        JSValue::number(&ctx, 1.0),
        JSValue::number(&ctx, 2.0),
        JSValue::number(&ctx, 3.0),
     ]
).unwrap();
assert_eq!(array.as_string().unwrap(), "1,2,3");
```

### Callbacks

```rust
use rust_jsc::{JSContext, JSFunction, JSObject, JSValue};

#[callback]
fn log_info(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let message = arguments.get(0).unwrap().as_string().unwrap();
    println!("INFO: {}", message);

    Ok(JSValue::undefined(&ctx))
}

let ctx = JSContext::new();
let global_object = ctx.global_object();

let object = JSObject::new(&ctx);
let attributes = PropertyDescriptorBuilder::new()
    .writable(true)
    .configurable(true)
    .enumerable(true)
    .build();
let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
object
    .set_property("log", &function, attributes)
    .unwrap();

global_object
    .set_property("console", &object, attributes)
    .unwrap();

let result = ctx.evaluate_script("console.log('Hello, World!')", None);
assert!(result.is_ok());
```

### Synthetic Modules

```rust
use rust_jsc::{
    callback, module_evaluate, module_fetch, module_import_meta, module_resolve,
    JSContext, JSFunction, JSObject, JSResult, JSString, JSStringProctected, JSValue, JSPromise,
    PropertyDescriptorBuilder, JSModuleLoader, PropertyDescriptor,
};

#[module_resolve]
fn module_loader_resolve(
    _ctx: JSContext,
    key: JSValue,
    _referrer: JSValue,
    _script_fetcher: JSValue,
) -> JSStringProctected {
    // referrer is the importing module. script_fetcher is currently undefined
    // for API callbacks on the rebased WebKit backend.
    JSStringProctected::from("@rust-jsc")
}

#[module_evaluate]
fn module_loader_evaluate(
    ctx: JSContext,
    key: JSValue,
) -> JSValue {

    // Module Loader Evaluate
    // is called only when evaluating Synthetic Modules
    let object = JSObject::new(&ctx);
    let keydata = JSValue::string(&ctx, "name");
    let value = JSValue::string(&ctx, "John Doe");
    object.set(&keydata, &value, PropertyDescriptor::default()).unwrap();

    object.into()
}

#[module_fetch]
fn module_loader_fetch(
    ctx: JSContext,
    key: JSValue,
    attributes_value: JSValue,
    script_fetcher: JSValue,
) -> JSStringProctected {
    // Module Loader Fetch
    // Fetch the content from file or network. attributes_value and
    // script_fetcher are currently undefined for API callbacks.
    JSStringProctected::from("let name = 'Kedojs'; export default name;")
}

#[module_import_meta]
fn module_loader_create_import_meta_properties(
    ctx: JSContext,
    key: JSValue,
    script_fetcher: JSValue,
) -> JSObject {
    // script_fetcher is currently undefined for API callbacks.

    let object = JSObject::new(&ctx);
    object.set_property("url", &key, Default::default()).unwrap();
    object
}

fn main() {
    let ctx = JSContext::new();
    let global_object = ctx.global_object();

    let module_loader = JSModuleLoader {
        // Disable the builtin file system loader
        disableBuiltinFileSystemLoader: true,
        moduleLoaderResolve: Some(module_loader_resolve),
        moduleLoaderEvaluate: Some(module_loader_evaluate),
        moduleLoaderFetch: Some(module_loader_fetch),
        moduleLoaderCreateImportMetaProperties: Some(
            module_loader_create_import_meta_properties,
        ),
    };
    ctx.set_module_loader(module_loader);

    let result = ctx.evaluate_module("./test.js");
    assert!(result.is_ok());
}
```

Synthetic module evaluation callbacks return an object whose own string-named
properties become module exports. A `"default"` property is used as the default
export. Symbol properties are ignored.

`JSContext::link_and_evaluate_module()` returns the JavaScript `Promise` created
by JavaScriptCore's module evaluator. Use `evaluate_module()` or
`evaluate_module_from_source()` when you want the current synchronous wrapper
that drains microtasks and reports startup exceptions through `JSResult`.

## Supported Platforms

Table below shows the supported platforms:

| Platform | Arch | Target | Supported | 
|----------|------|--------|-----------|
| macOS    | x86_64 | x86_64-apple-darwin | ✅ |
| macOS    | aarch64 | aarch64-apple-darwin | ✅ |
| Linux    | x86_64 | x86_64-unknown-linux-gnu | ✅ |
| Linux    | aarch64 | aarch64-unknown-linux-gnu | ✅ |
| Linux    | x86_64 | x86_64-unknown-linux-musl | ✅ |
| Linux    | aarch64 | aarch64-unknown-linux-musl | ✅ |
| Windows  | x86_64 | x86_64-pc-windows-msvc | ❌ |

## JavaScriptCore Builds

`rust_jsc` uses the `rust_jsc_sys` build script to locate and link the Kedo
WebKit JavaScriptCore build. The default path is static: with no configuration,
the build script downloads a prebuilt `libjsc-<target>.a.gz` archive from the
rust-jsc GitHub release mirror and links `libJavaScriptCore.a`, `libWTF.a`, and
`libbmalloc.a`; it also links `libJavaScriptCoreJIT.a` when a target produces a
separate JIT archive.

For local development against the bundled WebKit checkout:

```bash
make build-jsc-static
RUST_JSC_BUILD_MODE=static \
  RUST_JSC_CUSTOM_BUILD_PATH="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static/lib" \
  cargo test --lib -- --test-threads=1
```

Linux release archives are built in Docker and include bundled static
`libstdc++`, ICU, and `libatomic` archives. Local Linux source builds may link
those system dependencies dynamically when the static `.a` files are not present
beside the local JSC archives; that is intended for development and does not
change the default release archive path.

For every build mode and environment variable, see
[rust-jsc/sys/README.md](sys/README.md).

## FAQ

### How do I build JavaScriptCore locally?

Use the direct CMake/JSCOnly Makefile target. The default local build is static.

```bash
make build-jsc-static
make jsc-smoke
make test-local-jsc
```

To build and link the macOS framework layout instead:

```bash
make build-jsc-framework
RUST_JSC_BUILD_MODE=framework \
  RUST_JSC_CUSTOM_BUILD_PATH="$PWD/WebKit/WebKitBuild/RustJSC/JSCOnly/Release/lib" \
  cargo test
```

Linux release archives are produced with Docker:

```bash
make build-docker-jsc
make build-docker-jsc-musl
```

The complete build configuration reference lives in
[rust-jsc/sys/README.md](sys/README.md).

> :warning: This crate uses a custom [Kedo WebKit](https://github.com/kevincaicedo/Kedo-WebKit)
> fork. Stock system JavaScriptCore builds usually do not export the APIs that
> rust-jsc needs.

### How do I troubleshoot linking problems?

For static local builds, first confirm `RUST_JSC_CUSTOM_BUILD_PATH` points at
the directory containing `libJavaScriptCore.a`, `libWTF.a`, and `libbmalloc.a`.
For framework or dynamic builds, set the platform loader path if needed:

```bash
# macOS
export DYLD_LIBRARY_PATH=/path/to/jsc/lib:$DYLD_LIBRARY_PATH
```

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/jsc/lib:$LD_LIBRARY_PATH
```

More troubleshooting notes are in [rust-jsc/sys/README.md](sys/README.md).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
