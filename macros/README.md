# rust_jsc_macros

`rust_jsc_macros` contains the procedural callback macros used by
`rust_jsc`. Most users should depend on `rust_jsc` and import these macros
through its public re-exports.

The macros generate JavaScriptCore-compatible callback wrappers with:

- compile-time signature validation for supported callback roles
- panic boundaries before returning across the C ABI
- typed argument extraction through `FromJSValue` and `TryFromJSValue`
- typed return conversion through `IntoJSResult` and `IntoJSValue`

Example:

```rust
use rust_jsc::{callback, JSContext, JSResult, JSValue};

#[callback]
fn add_one(ctx: JSContext, value: JSValue) -> JSResult<i32> {
    Ok(value.as_number()? as i32 + 1)
}
```

The complete callback signature guide lives in the
[rust-jsc macro guide](https://github.com/kevincaicedo/rust-jsc/blob/main/docs/macro-guide.md).
