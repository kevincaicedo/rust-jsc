use rust_jsc::{callback, constructor, JSContext, JSObject, JSResult, JSValue};

#[callback]
fn direct_string(
    _ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
) -> &'static str {
    "hello"
}

#[callback]
fn fallible_bool(
    _ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
) -> JSResult<bool> {
    Ok(true)
}

#[callback]
fn undefined_return(_ctx: JSContext, _function: JSObject, _this: JSObject) {}

#[constructor]
fn object_constructor(ctx: JSContext, _constructor: JSObject) -> JSResult<JSObject> {
    Ok(JSObject::new(&ctx))
}

#[constructor]
fn value_constructor(ctx: JSContext, _constructor: JSObject) -> JSValue {
    JSValue::undefined(&ctx)
}

fn main() {
    let _ = direct_string;
    let _ = fallible_bool;
    let _ = undefined_return;
    let _ = object_constructor;
    let _ = value_constructor;
}
