use rust_jsc::{callback, JSContext, JSObject, JSResult, JSValue, Rest};

#[callback]
fn typed_callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    name: Option<String>,
    numbers: Rest<f64>,
) -> JSResult<JSValue> {
    let total = numbers.iter().sum::<f64>() + name.map(|name| name.len()).unwrap_or(0) as f64;
    Ok(JSValue::number(&ctx, total))
}

fn main() {
    let _ = typed_callback;
}
