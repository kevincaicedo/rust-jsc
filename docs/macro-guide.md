# rust-jsc Macro Guide

`rust-jsc` macros adapt safe Rust functions to JavaScriptCore C callbacks. The
macro layer should be predictable: unsupported signatures fail at compile time,
runtime failures become JavaScript errors where the C ABI has an exception slot,
and panics never cross FFI.

## Function Callbacks

Use `#[callback]` for JavaScript functions created with
`JSFunction::callback`.

```rust
use rust_jsc::{callback, JSResult};

#[callback]
fn greet(name: Option<String>) -> JSResult<String> {
    let name = name.unwrap_or_else(|| "world".to_string());
    Ok(format!("hello {name}"))
}
```

When the callback needs JavaScriptCore ABI roles, request them explicitly with
role-marker parameters before JavaScript arguments:

```rust
use rust_jsc::{
    callback, CallbackContext, CallbackFunction, JSResult, JSValue, ThisObject,
};

#[callback]
fn method(
    ctx: CallbackContext,
    this: ThisObject,
    function: CallbackFunction,
    amount: f64,
) -> JSResult<JSValue> {
    let _ = this.as_object();
    let _ = function.as_object();
    Ok(JSValue::number(&ctx, amount))
}
```

Supported typed arguments use `TryFromJSValue`:

| Rust type | JavaScript conversion |
| --- | --- |
| `JSValue` | borrowed JS value wrapper clone |
| `JSObject` | `ToObject` through JavaScriptCore |
| `JSString` | `ToString` through JavaScriptCore |
| `String` | `ToString`, then UTF-8 Rust string |
| `bool` | JavaScript truthiness |
| `f64` | JavaScript number conversion |
| `i32`, `u32`, `usize` | checked finite integer conversion |
| `JSArray` | exact JavaScript Array check |
| `JSFunction` | object plus callable check |
| `JSTypedArray` | typed-array check |
| `Option<T>` | missing, `undefined`, or `null` become `None` |
| `Rest<T>` | all remaining arguments converted with `T: TryFromJSValue` |

Callback returns use `IntoJSResult`. Returning `JSResult<JSValue>` remains
valid, but typed callbacks may also return `JSValue`, `JSObject`, `JSString`,
`String`, `&'static str`, `bool`, numbers, `Option<T>`, `()`, or `JSResult<T>`
where `T: IntoJSValue`. Conversion failures become JavaScript exceptions.

The legacy raw form remains supported:

```rust
use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue};

#[callback]
fn raw(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    Ok(arguments
        .first()
        .cloned()
        .unwrap_or_else(|| JSValue::undefined(&ctx)))
}
```

Typed callback arguments are parsed directly from the raw JavaScriptCore
argument slice. The macro allocates a `Vec<JSValue>` only for the exact legacy
`&[JSValue]` form. `Rest<T>` must be the last typed argument. The exact legacy
ABI prefix `(JSContext, JSObject, JSObject, ...)` still means
context/function/this; new code should use `CallbackContext`,
`CallbackFunction`, and `ThisObject` to make those roles explicit.

## Constructors

Use `#[constructor]` with typed arguments for the normal safe path:

```rust
use rust_jsc::{constructor, CallbackContext, JSObject, JSResult, JSValue, Rest};

#[constructor]
fn new_counter(
    ctx: CallbackContext,
    initial: Option<f64>,
    labels: Rest<String>,
) -> JSResult<JSObject> {
    let object = JSObject::new(&ctx);
    object.set_property(
        "value",
        &JSValue::number(&ctx, initial.unwrap_or_default()),
        Default::default(),
    )?;
    object.set_property(
        "labelCount",
        &JSValue::number(&ctx, labels.len() as f64),
        Default::default(),
    )?;
    Ok(object)
}
```

Constructor argument conversion uses the same `TryFromJSValue`, `Option<T>`,
and `Rest<T>` rules as `#[callback]`. Constructor returns also use
`IntoJSResult`; returning `JSResult<JSObject>` or `JSResult<JSValue>` is the
normal path. The low-level legacy
`(JSContext, JSObject, &[JSValue])` form remains available when a constructor
needs raw argument inspection. Use `ConstructorObject` before JavaScript
arguments when new constructor code needs the invoked constructor object.

## Has Instance

Use `#[has_instance]` for custom `instanceof` behavior:

```rust
use rust_jsc::{has_instance, JSContext, JSObject, JSResult, JSValue};

#[has_instance]
fn is_counter(
    _ctx: JSContext,
    _constructor: JSObject,
    possible_instance: JSValue,
) -> JSResult<bool> {
    Ok(possible_instance.is_object())
}
```

## Class Lifecycle

Use `#[initialize]` and `#[finalize]` for low-level JavaScriptCore class
lifecycle hooks:

```rust
use rust_jsc::{finalize, initialize, JSContext, JSObject, PrivateData};

#[initialize]
fn initialize(_ctx: JSContext, _object: JSObject) {}

#[finalize]
fn finalize(_data: PrivateData) {}
```

Both wrappers validate their signatures at compile time, guard null raw
JavaScriptCore inputs, and catch Rust panics before returning to the C ABI.
Custom finalizers receive the raw private-data pointer and are responsible for
any ownership policy they impose. Prefer the default Rust-owned private-data
finalizer unless a class needs custom teardown behavior.

## Module Loader Macros

Prefer the typed module-loader macros for new custom loaders. They expose
owned Rust strings, structured import types, and typed return traits while
still expanding to JavaScriptCore callback pointers for `ModuleLoaderBuilder`.

```rust
use rust_jsc::{
    module_fetcher, module_import_meta_provider, module_resolver, JSContext,
    JSObject, JSResult, JSValue, ModuleImportType, ModuleSource,
};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    referrer: Option<String>,
) -> JSResult<Option<String>> {
    Ok(Some(referrer.unwrap_or(specifier)))
}

#[module_fetcher]
fn fetch(
    _ctx: JSContext,
    key: String,
    import_type: ModuleImportType,
) -> JSResult<Option<ModuleSource>> {
    let source = match import_type {
        ModuleImportType::Json => ModuleSource::Json(format!("{{\"key\":{key:?}}}")),
        ModuleImportType::WebAssembly => return Ok(None),
        ModuleImportType::Unknown | ModuleImportType::JavaScript => {
            ModuleSource::JavaScript(format!("export default {key:?};"))
        }
    };
    Ok(Some(source))
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let object = JSObject::new(&ctx);
    object.set_property("url", &JSValue::string(&ctx, key), Default::default())?;
    Ok(Some(object))
}
```

`#[module_resolver]` expects
`(JSContext, String, Option<String>)` and accepts return values implementing
`IntoModuleResolveResult`, including `String`, `Option<String>`,
`JSStringProtected`, and `JSResult<T>` where `T: IntoModuleResolveResult`.

`#[module_fetcher]` expects `(JSContext, String, ModuleImportType)` and targets
the preferred `fetch_source` callback slot. It accepts `ModuleSource`,
`JSModuleSource`, optional variants, and `JSResult<T>` where
`T: IntoModuleSourceResult`.

`#[module_import_meta_provider]` expects `(JSContext, String)` and accepts
`JSObject`, `Option<JSObject>`, or `JSResult<T>` where
`T: IntoImportMetaResult`.

The lower-level module-loader macros remain available for the raw C ABI. They
mirror JavaScriptCore callback arguments as `JSValue` and return
`JSStringProtected`, `JSObject`, or `JSValue` depending on the hook.

```rust
use rust_jsc::{
    module_fetch, module_import_meta, module_resolve, JSContext, JSObject,
    JSStringProtected, JSValue,
};

#[module_resolve]
fn resolve(
    _ctx: JSContext,
    key: JSValue,
    _referrer: JSValue,
    _fetcher: JSValue,
) -> JSStringProtected {
    let key = key
        .as_string()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "module".to_string());
    JSStringProtected::from(key)
}

#[module_fetch]
fn fetch(
    _ctx: JSContext,
    key: JSValue,
    _attributes: JSValue,
    _fetcher: JSValue,
) -> JSStringProtected {
    let key = key
        .as_string()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "module".to_string());
    JSStringProtected::from(format!("export default {key:?};"))
}

#[module_import_meta]
fn import_meta(ctx: JSContext, key: JSValue, _fetcher: JSValue) -> JSObject {
    let object = JSObject::new(&ctx);
    let _ = object.set_property("url", &key, Default::default());
    object
}
```

The JavaScriptCore module-loader C ABI has no exception out-pointer. Typed
loader macros still accept `JSResult` so user code can be fallible, but
conversion failures, returned errors, and panics are contained by returning a
null callback result to JavaScriptCore.

## Inspector Macros

`#[inspector_callback]` receives a borrowed message string for the callback
duration only. Invalid UTF-8 from the C layer is converted lossily before the
Rust function is called. New embedding code should install the generated
callback through `JSContext::inspector_session()` so the connection has a
drop-time disconnect owner.

```rust
use rust_jsc::inspector_callback;

#[inspector_callback]
fn frontend(message: &str) {
    eprintln!("{message}");
}
```

`#[inspector_pause_event_callback]` receives a borrowed context and a typed
pause event. It is a pump notification, not a place to run arbitrary JavaScript
or destroy the VM.

## Stability Rules

All documented macros:

- reject async, const, unsafe, method receiver, and unsupported signature forms
- use generated private local names
- catch Rust panics before returning to JavaScriptCore
- guard null raw callback inputs
- write JavaScript exceptions only when the ABI provides an exception pointer

The macro contract is part of the public API. Any future ergonomic macro should
generate calls into the safe builders and conversion traits rather than
duplicating ownership or exception logic.
