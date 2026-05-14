#![allow(unused_imports)]

use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue};

struct Callbacks;

impl Callbacks {
    #[callback]
    fn method(
        &self,
        ctx: JSContext,
        _function: JSObject,
        _this: JSObject,
    ) -> JSResult<JSValue> {
        Ok(JSValue::undefined(&ctx))
    }
}

fn main() {}
