# Embedding rust-jsc

This guide shows the minimal rust-jsc host shape for an embedding runtime. It
keeps JavaScriptCore ownership visible: the host owns the global context, Rust
callbacks receive borrowed handles, module evaluation returns promises, and
microtasks run only at explicit host checkpoints.

## Minimal Context

`JSContext::new()` returns an owned global context. The borrowed `JSContext`
view is used by callbacks and methods, while the owned handle releases the
underlying global context in `Drop`.

```rust
use rust_jsc::{JSContext, JSResult};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let value = ctx.evaluate_script("'hello from rust-jsc'", None)?;

    assert_eq!(value.as_string()?.to_string(), "hello from rust-jsc");
    Ok(())
}
```

Context, value, object, promise, inspector, and typed-array handles are
JavaScriptCore-thread-affine. Move work across threads as Rust-owned messages
or buffers, then touch JavaScriptCore again from the owning JS thread.

## Host Callbacks

Use typed callback macros for host functions. The wrapper catches Rust panics
before returning across the C ABI and converts JavaScript arguments through
`TryFromJSValue`.

```rust
use rust_jsc::{callback, JSContext, JSFunction, JSResult, JSValue};

#[callback]
fn add(left: f64, right: f64) -> f64 {
    left + right
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let function = JSFunction::callback(&ctx, Some("add"), Some(add));
    let function_value: JSValue = function.into();
    ctx.global_object()
        .set_property("add", &function_value, Default::default())?;

    let result = ctx.evaluate_script("add(20, 22)", None)?;
    assert_eq!(result.as_number()?, 42.0);
    Ok(())
}
```

When a callback needs JavaScriptCore roles, request them explicitly with
`CallbackContext`, `CallbackFunction`, `ThisObject`, or `ConstructorObject`.
Ordinary typed parameters remain JavaScript arguments.

## Modules

The default file module loader resolves filesystem modules, JSON modules,
WebAssembly modules, dynamic imports, and `import.meta.url`.

```rust
use rust_jsc::{JSContext, JSResult, ModuleLoader};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let promise = ctx.evaluate_module("/absolute/path/to/main.js")?;
    assert!(promise.is_object());

    ctx.run_deferred_work();
    ctx.run_microtasks();
    Ok(())
}
```

Module evaluation returns a JavaScript promise. The host decides when to call
`run_deferred_work()` and `run_microtasks()`; rust-jsc does not hide event-loop
policy inside evaluation.

## Deferred Promises

Use `Promise::new_pending()` or `JSPromise::new_pending()` when Rust needs to
resolve a JavaScript promise later. The resolver protects its resolve and
reject functions with RAII guards and remains context-affine.

```rust
use rust_jsc::{JSContext, JSResult, JSValue, Promise};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let (_promise, resolver) = Promise::new_pending(&ctx)?;
    resolver.resolve(None, &[JSValue::string(&ctx, "ready")])?;

    ctx.run_microtasks();
    Ok(())
}
```

Store resolver handles only in JS-thread-owned runtime state unless a higher
level runtime layer owns a safe cross-thread handoff protocol.

Use `JSContext::set_unhandled_rejection_handler(&function)` when the host needs
to observe unhandled rejections. Keep the returned `UnhandledRejectionHandler`
with runtime state for as long as the callback should remain protected.

## Inspector Sessions

`JSContext::inspector_session()` creates the direct JavaScriptCore inspector
safe path. A session owns the frontend attachment and disconnects on drop.

```rust
use rust_jsc::{inspector_callback, JSContext, JSResult};

#[inspector_callback]
fn on_message(message: &str) {
    println!("{message}");
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let session = ctx
        .inspector_session()
        .on_message(on_message)
        .connect()?;

    session.send_message(r#"{"id":1,"method":"Runtime.enable"}"#)?;
    Ok(())
}
```

Inspector message callbacks receive borrowed UTF-8 bytes for the callback
duration. Copy a message before storing it outside the callback.

## Runtime Shape

A small host loop usually owns:

- one `JSGlobalContext` on the JS thread
- a module loader registered during startup
- host callback functions installed on the global object or through modules
- Rust-owned queues for timers, I/O completions, and deferred promise work
- explicit checkpoints for deferred work, microtasks, and inspector pump events

KedoJS adds policy above these primitives. rust-jsc keeps the binding surface
reusable and does not own the runtime event-loop contract.
