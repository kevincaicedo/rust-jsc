#![allow(unused_imports)]

use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue};

#[callback]
fn callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _message: &str,
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn main() {}
