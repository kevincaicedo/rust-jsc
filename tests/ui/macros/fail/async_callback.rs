#![allow(unused_imports)]

use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue};

#[callback]
async fn callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn main() {}
