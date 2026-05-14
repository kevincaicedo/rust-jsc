use rust_jsc::{
    callback, constructor, CallbackContext, CallbackFunction, ConstructorObject,
    JSObject, JSResult, JSValue, ThisObject,
};

#[callback]
fn add(left: f64, right: f64) -> f64 {
    left + right
}

#[callback]
fn method(
    ctx: CallbackContext,
    this: ThisObject,
    function: CallbackFunction,
    value: f64,
) -> JSResult<JSValue> {
    let _ = this.as_object();
    let _ = function.as_object();
    Ok(JSValue::number(&ctx, value))
}

#[callback]
fn raw(ctx: CallbackContext, arguments: &[JSValue]) -> JSResult<JSValue> {
    Ok(arguments
        .first()
        .cloned()
        .unwrap_or_else(|| JSValue::undefined(&ctx)))
}

#[constructor]
fn new_object(
    ctx: CallbackContext,
    constructor: ConstructorObject,
    label: Option<String>,
) -> JSResult<JSObject> {
    let _ = constructor.as_object();
    let object = JSObject::new(&ctx);
    object.set_property(
        "label",
        &JSValue::string(&ctx, label.unwrap_or_default()),
        Default::default(),
    )?;
    Ok(object)
}

fn main() {
    let _ = add;
    let _ = method;
    let _ = raw;
    let _ = new_object;
}
