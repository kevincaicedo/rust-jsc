#![allow(unused_imports)]

use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue, Rest};

#[callback]
fn callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _rest: Rest<JSValue>,
    _tail: JSValue,
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn main() {}
