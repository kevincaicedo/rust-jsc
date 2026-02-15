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
rust_jsc = { version = "0.4.2" }
```

# Usage

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
use crate::{JSArrayBuffer, JSContext, JSTypedArray, JSTypedArrayType};

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
    JSContext, JSFunction, JSObject, JSResult, JSString, JSStringRetain, JSValue, JSPromise,
    PropertyDescriptorBuilder, JSModuleLoader, PropertyDescriptor,
};

#[module_resolve]
fn module_loader_resolve(
    ctx: JSContext,
    key: JSValue,
    referrer: JSValue,
    script_fetcher: JSValue,
) -> JSStringRetain {
    JSStringRetain::from("@rust-jsc")
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
) -> JSStringRetain {
    // Module Loader Fetch
    // fetch the content from file or network
    JSStringRetain::from("let name = 'Kedojs'; export default name;")
}

#[module_import_meta]
fn module_loader_create_import_meta_properties(
    ctx: JSContext,
    key: JSValue,
    script_fetcher: JSValue,
) -> JSObject {
    let key_value = key.as_string().unwrap();

    let object = JSObject::new(&ctx);
    object.set_property("url", &key, Default::default()).unwrap();
    object
}

fn main() {
    let ctx = JSContext::new();
    let global_object = ctx.global_object();

    let module_loader = JSAPIModuleLoader {
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


## FAQ

### How do I build the static libraries?

By default, this library will try to download the static libraries from the GitHub mirror. If you want to build the static libraries yourself, you can clone the [rust-jsc repo](https://github.com/kevincaicedo/rust-jsc) and build the Docker image from the Dockerfile. It will build the static libraries for you and copy them to the provided path.

```bash
DOCKER_BUILDKIT=1 docker build -o ./.libs -t $(IMAGE_NAME) .
```

This command will only work on Linux. For macOS, you should build the JavaScriptCore static libraries by running the following command from the Makefile:

```bash
make build-jsc
```

Then set the `RUST_JSC_CUSTOM_BUILD_PATH` environment variable to the path of the static libraries.

in order to archive the static libraries, you can use the following command:

```bash
# For macOS
make archive platform=aarch64-apple-darwin
make archive platform=x86_64-apple-darwin

# For Linux
make archive platform=aarch64-unknown-linux-gnu
make archive platform=x86_64-unknown-linux-gnu
```

> :warning: **Keep in mind this lib use a custom version of [WebKit](https://github.com/kevincaicedo/Kedo-WebKit) to generate the bindings. this version of WebKit is a fork of the original WebKit with some patches to support esmodules and other features.**

### How do I troubleshoot linking problems?

If you encounter any problems linking the static libraries, try setting the following environment variables:

```bash
# For macOS
# Example path to the JavaScriptCore static libraries
DYLD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$DYLD_LIBRARY_PATH
```

```bash
# For Linux
# Example path to the JavaScriptCore static libraries
LD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$LD_LIBRARY_PATH
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
