use rust_jsc_sys::{JSObjectMakeDate, JSValueRef};

use crate::{JSContext, JSDate, JSError, JSObject, JSResult, JSValue};

impl JSDate {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    pub fn new_date(ctx: JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();

        let result = unsafe {
            JSObjectMakeDate(ctx.inner, args.len(), args.as_ptr(), &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
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
