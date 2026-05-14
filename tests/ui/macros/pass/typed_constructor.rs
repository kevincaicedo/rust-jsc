use rust_jsc::{constructor, JSContext, JSObject, JSResult, JSValue, Rest};

#[constructor]
fn typed_constructor(
    ctx: JSContext,
    _constructor: JSObject,
    name: String,
    age: Option<f64>,
    tags: Rest<String>,
) -> JSResult<JSValue> {
    let object = JSObject::new(&ctx);
    object.set_property("name", &JSValue::string(&ctx, name), Default::default())?;
    object.set_property(
        "age",
        &JSValue::number(&ctx, age.unwrap_or_default()),
        Default::default(),
    )?;
    object.set_property(
        "tagCount",
        &JSValue::number(&ctx, tags.len() as f64),
        Default::default(),
    )?;
    Ok(object.into())
}

fn main() {
    let _ = typed_constructor;
}
