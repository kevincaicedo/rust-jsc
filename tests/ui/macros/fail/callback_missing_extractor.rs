use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue};

struct MissingExtractor;

#[callback]
fn callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _value: MissingExtractor,
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn main() {}
