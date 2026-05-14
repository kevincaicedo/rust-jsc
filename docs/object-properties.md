# Object Properties

`rust-jsc` has two object property layers:

- use `PropertyKey` and `JSObject::{set,get,has,delete}_property_by_key` for
  normal embedding code
- use the lower-level string/value/index methods when a binding needs to mirror
  one JavaScriptCore C API exactly

The high-level API keeps string names, JavaScript value keys, symbols, and array
indexes explicit while preserving JavaScriptCore exception behavior.

## Property Keys

`PropertyKey` is the safe path when an API accepts more than one key shape:

```rust
use rust_jsc::{
    JSContext, JSObject, JSResult, JSValue, PropertyDescriptor, PropertyKey,
};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let object = JSObject::new(&ctx);

    let name = JSValue::string(&ctx, "kedo");
    object.set_property_by_key(
        "name",
        &name,
        PropertyDescriptor::default(),
    )?;

    let index_value = JSValue::number(&ctx, 42.0);
    object.set_property_by_key(
        PropertyKey::index(0),
        &index_value,
        Default::default(),
    )?;

    let symbol = PropertyKey::symbol(&ctx, "runtime.id");
    let symbol_value = JSValue::string(&ctx, "rust-jsc");
    object.set_property_by_key(
        symbol.clone(),
        &symbol_value,
        Default::default(),
    )?;

    assert_eq!(object.get_property_by_key("name")?, name);
    assert_eq!(object.get_property_by_key(0_u32)?, index_value);
    assert_eq!(object.get_property_by_key(symbol)?, symbol_value);
    Ok(())
}
```

Value keys are checked against the object's context before crossing the C API.
If JavaScriptCore throws while converting a value key with `ToPropertyKey`, the
safe wrapper returns `Err(JSError)`.

## Descriptors

`PropertyDescriptor::builder()` is the intended API for static descriptors:

```rust
use rust_jsc::{JSContext, JSObject, JSResult, JSValue, PropertyDescriptor};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let object = JSObject::new(&ctx);
    let value = JSValue::number(&ctx, 1.0);

    let descriptor = PropertyDescriptor::builder()
        .read_only()
        .non_enumerable()
        .non_configurable()
        .build();

    object.set_property_by_key("version", &value, descriptor)?;
    Ok(())
}
```

Named builder methods are clearer for fixed policy:

- `read_only()` sets JavaScriptCore's read-only bit
- `non_enumerable()` hides the property from string-key enumeration
- `non_configurable()` prevents reconfiguration and deletion

Boolean setters remain available for runtime policy translation:

```rust
let descriptor = PropertyDescriptor::builder()
    .writable(can_write)
    .enumerable(show_in_enumeration)
    .configurable(can_reconfigure)
    .build();
```

Use `from_raw_attributes` and `raw_attributes` only when bridging an existing
JavaScriptCore attribute bitset.

## Property Names

`JSObject::get_property_names()` returns string property names through an RAII
iterator. The iterator releases JavaScriptCore's property-name array on drop and
is exact-size and fused, so callers can pre-size collections and keep polling
after exhaustion.

Symbol keys are intentionally not string property names. Store and reuse the
symbol key itself when a Rust embedding needs to access a symbol-keyed property.

## Exception Behavior

Property APIs return `JSResult` whenever JavaScriptCore can report an exception.
The wrappers preserve exceptions from:

- setter traps and accessor code
- value-key `ToPropertyKey` conversion
- delete/has/get operations using throwing value keys
- cross-context key or value misuse, reported before crossing FFI

String-only `has_property` mirrors JavaScriptCore's C API and returns `bool`.
Use `has_property_by_key` when code needs `JSResult<bool>` and context checking.
