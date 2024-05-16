use rust_jsc::{JSContext, callback, JSFunction, JSObject, JSResult, JSValue, PropertyDescriptorBuilder};

#[callback]
fn log_info(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let message = arguments.get(0).unwrap().as_string().unwrap();
    println!("INFO: {}", message);

    Ok(JSValue::undefined(&ctx))
}

fn main() {
    let ctx = JSContext::new();
    let global_object = ctx.global_object();

    let object = JSObject::new(&ctx);
    let attributes = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();
    let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
    object
        .set_property(&"log".into(), &function.into(), attributes)
        .unwrap();

    global_object
        .set_property(&"console".into(), &object.into(), attributes)
        .unwrap();

    let result = ctx.evaluate_script("console.log('Hello, World!')", None);
    assert!(result.is_ok());
}
