use rust_jsc_sys::{JSObjectMakeDate, JSValueRef};

use crate::{
    with_raw_value_refs, JSContext, JSDate, JSError, JSObject, JSResult, JSValue,
};

impl JSDate {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    pub fn new_date(ctx: JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `ctx.inner` is a live context and `arguments` points to
            // `argument_count` raw JS values for the duration of this call.
            unsafe {
                JSObjectMakeDate(ctx.inner, argument_count, arguments, &mut exception)
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::from_message(
                &ctx,
                "failed to create JavaScript Date object",
            ));
        }

        Ok(Self::new(JSObject::from_ref(result, ctx.inner)))
    }
}

impl From<JSDate> for JSObject {
    fn from(regexp: JSDate) -> Self {
        regexp.object
    }
}

impl From<JSObject> for JSDate {
    fn from(object: JSObject) -> Self {
        Self::new(object)
    }
}

impl std::fmt::Display for JSDate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Date: {:?}", self.object)
    }
}

impl From<JSValue> for JSDate {
    fn from(value: JSValue) -> Self {
        Self::new(value.as_object().unwrap())
    }
}

impl From<JSDate> for JSValue {
    fn from(date: JSDate) -> Self {
        date.object.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{JSContext, JSDate, JSValue};

    #[test]
    fn test_date_new_date() {
        let ctx = JSContext::new();
        let args = [
            JSValue::number(&ctx, 2026.0),
            JSValue::number(&ctx, 4.0),
            JSValue::number(&ctx, 23.0),
        ];
        let date = JSDate::new_date(*ctx.as_context(), &args).unwrap();
        assert!(date.object.is_object());
    }
}
