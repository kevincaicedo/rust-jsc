use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeError, JSObjectMakeTypeError, JSValueRef};

use crate::{JSContext, JSError, JSObject, JSResult, JSString, JSValue};

impl JSError {
    /// Creates a new `JSError` object.
    /// This is the same as `new Error()`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The JavaScript context.
    /// * `args` - The arguments to pass to the error constructor.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_jsc::{JSContext, JSError};
    ///
    /// let ctx = JSContext::new();
    /// let error = JSError::new(&ctx, &[]).unwrap();
    /// assert_eq!(error.name().unwrap().to_string(), "Error");
    /// ```
    ///
    /// # Returns
    ///
    /// A new `JSError` object.
    pub fn new(ctx: &JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();

        let result = unsafe {
            JSObjectMakeError(ctx.inner, args.len(), args.as_ptr(), &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self::from(JSObject::from_ref(result, ctx.inner)))
    }

    /// Creates a new `JSError` object with the given message.
    /// This is the same as `new TypeError(message)`
    ///
    /// # Arguments
    ///
    /// * `ctx` - The JavaScript context.
    /// * `message` - The error message.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_jsc::{JSContext, JSError};
    ///
    /// let ctx = JSContext::new();
    /// let error = JSError::new_typ(&ctx, "test error").unwrap();
    /// assert_eq!(error.name().unwrap().to_string(), "TypeError");
    /// assert_eq!(error.message().unwrap().to_string(), "test error");
    /// ```
    ///
    /// # Returns
    ///
    /// A new `JSError` of type `TypeError`.
    pub fn new_typ(ctx: &JSContext, message: impl Into<JSString>) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeTypeError(ctx.inner, message.into().inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self::from(JSObject::from_ref(result, ctx.inner)))
    }

    pub fn new_typ_raw(ctx: &JSContext, message: impl Into<JSString>) -> JSValueRef {
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeTypeError(ctx.inner, message.into().inner, &mut exception)
        };

        if !exception.is_null() {
            return exception;
        }

        result
    }

    pub fn with_message(ctx: &JSContext, message: impl Into<JSString>) -> JSResult<Self> {
        let args = [JSValue::string(ctx, message)];
        Self::new(ctx, &args)
    }

    pub fn name(&self) -> JSResult<JSString> {
        self.object.get_property("name")?.as_string()
    }

    pub fn message(&self) -> JSResult<JSString> {
        self.object.get_property("message")?.as_string()
    }

    pub fn cause(&self) -> JSResult<JSValue> {
        self.object.get_property("cause")
    }

    pub fn stack(&self) -> JSResult<JSString> {
        self.object.get_property("stack")?.as_string()
    }

    pub fn set_cause(&self, cause: &JSValue) -> JSResult<()> {
        self.object.set_property("cause", cause, Default::default())
    }

    pub fn set_stack(&self, stack: &JSValue) -> JSResult<()> {
        self.object.set_property("stack", stack, Default::default())
    }
}

impl std::fmt::Display for JSError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "JavaScript error: {:?}", self.message().unwrap())
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

impl Deref for JSError {
    type Target = JSValue;

    fn deref(&self) -> &JSValue {
        &self.object.value
    }
}

impl From<JSError> for JSValue {
    fn from(error: JSError) -> Self {
        error.object.into()
    }
}

impl From<JSError> for JSObject {
    fn from(error: JSError) -> Self {
        error.object
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error() {
        let ctx = JSContext::new();
        let error = JSError::new_typ(&ctx, "test error").unwrap();
        assert_eq!(error.name().unwrap().to_string(), "TypeError");
        assert_eq!(error.message().unwrap().to_string(), "test error");

        let global_object = ctx.global_object();
        global_object
            .set_property("myError", &error, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("myError instanceof TypeError", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_boolean(), true);
    }

    #[test]
    fn test_error() {
        let ctx = JSContext::new();
        let error = JSError::with_message(&ctx, "test error").unwrap();
        assert_eq!(error.name().unwrap().to_string(), "Error");
        assert_eq!(error.message().unwrap().to_string(), "test error");

        let global_object = ctx.global_object();
        global_object
            .set_property("myError", &error, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("myError instanceof Error", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_boolean(), true);
    }
}
