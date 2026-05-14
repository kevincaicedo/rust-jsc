#![allow(unused_imports)]

use rust_jsc::{has_instance, JSContext, JSObject, JSResult, JSValue};

#[has_instance]
fn has_instance(
    _ctx: JSContext,
    _constructor: JSObject,
    _possible_instance: JSValue,
) -> JSResult<JSValue> {
    Ok(_possible_instance)
}

fn main() {}
