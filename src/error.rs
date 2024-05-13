use rust_jsc_sys::{JSObjectMakeError, JSValueRef};

use crate::{JSContext, JSError, JSObject, JSResult, JSString, JSValue};

impl JSError {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    pub fn new_error(ctx: &JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();

        let result = unsafe {
            JSObjectMakeError(ctx.inner, args.len(), args.as_ptr(), &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self::new(JSObject::from_ref(result, ctx.inner)))
    }

   pub fn with_message(ctx: &JSContext, message: JSString) -> JSResult<Self> {
        let args = [JSValue::string(ctx, message)];
        Self::new_error(ctx, &args)
    }

    pub fn name(&self) -> JSResult<JSString> {
        self.object.get_property(&"name".into())?.as_string()
    }

    pub fn message(&self) -> JSResult<JSString> {
        self.object.get_property(&"message".into())?.as_string()
    }

    pub fn cause(&self) -> JSResult<JSValue> {
        self.object.get_property(&"cause".into())
    }

    pub fn stack(&self) -> JSResult<JSString> {
        self.object.get_property(&"stack".into())?.as_string()
    }
}

impl std::fmt::Display for JSError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "JavaScript error: {:?}", self.object)
    }
}

impl std::error::Error for JSError {}

impl From<JSValue> for JSError {
    fn from(value: JSValue) -> Self {
        Self {
            object: value.as_object().unwrap(),
        }
    }
}

impl From<JSObject> for JSError {
    fn from(object: JSObject) -> Self {
        Self { object }
    }
}

impl From<JSError> for JSValueRef {
    fn from(error: JSError) -> Self {
        error.object.value.inner
    }
}
