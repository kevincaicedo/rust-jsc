use rust_jsc_sys::{JSObjectMakeRegExp, JSValueRef};

use crate::{JSContext, JSError, JSObject, JSRegExp, JSResult, JSValue};

impl JSRegExp {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    pub fn new_regexp(ctx: JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();

        let result = unsafe {
            JSObjectMakeRegExp(ctx.inner, args.len(), args.as_ptr(), &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self::new(JSObject::from_ref(result, ctx.inner)))
    }
}

impl From<JSRegExp> for JSObject {
    fn from(regexp: JSRegExp) -> Self {
        regexp.object
    }
}

impl From<JSRegExp> for JSValue {
    fn from(regexp: JSRegExp) -> Self {
        regexp.object.into()
    }
}
