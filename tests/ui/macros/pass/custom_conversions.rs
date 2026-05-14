use rust_jsc::{
    callback, constructor, IntoJSValue, JSContext, JSError, JSObject, JSResult,
    JSValue, TryFromJSValue,
};

struct Label(String);

impl TryFromJSValue for Label {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        value.as_string().map(|label| Self(label.to_string()))
    }
}

struct CallbackMessage(String);

impl IntoJSValue for CallbackMessage {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::string(ctx, self.0))
    }
}

#[callback]
fn custom_arg_and_return(
    _ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    label: Label,
) -> CallbackMessage {
    CallbackMessage(label.0)
}

#[constructor]
fn custom_constructor(
    ctx: JSContext,
    _constructor: JSObject,
    label: Label,
) -> JSResult<JSObject> {
    if label.0.is_empty() {
        return Err(JSError::new_typ(&ctx, "empty label")?);
    }

    let object = JSObject::new(&ctx);
    object.set_property(
        "label",
        &JSValue::string(&ctx, label.0),
        Default::default(),
    )?;
    Ok(object)
}

fn main() {
    let _ = custom_arg_and_return;
    let _ = custom_constructor;
}
