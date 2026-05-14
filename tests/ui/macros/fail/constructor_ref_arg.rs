#![allow(unused_imports)]

use rust_jsc::{constructor, JSContext, JSObject, JSResult, JSValue};

#[constructor]
fn constructor(
    ctx: JSContext,
    _constructor: JSObject,
    _name: &str,
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn main() {}
