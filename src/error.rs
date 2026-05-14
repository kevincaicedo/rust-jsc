use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeError, JSObjectMakeTypeError, JSObjectRef, JSValueRef};

use crate::{
    with_raw_value_refs, JSContext, JSError, JSObject, JSResult, JSString, JSValue,
};

impl JSError {
    pub(crate) fn from_message(ctx: &JSContext, message: impl Into<JSString>) -> Self {
        match Self::with_message(ctx, message) {
            Ok(error) | Err(error) => error,
        }
    }

    fn fallback(ctx: &JSContext, name: &str, message: &str) -> Self {
        let object = JSObject::new(ctx);
        let name = JSValue::string(ctx, name);
        let message = JSValue::string(ctx, message);

        let _ = object.set_property("name", &name, Default::default());
        let _ = object.set_property("message", &message, Default::default());

        Self { object }
    }

    fn from_exception(value: JSValue) -> Self {
        if value.is_object() {
            return Self {
                object: JSObject {
                    inner: value.inner as JSObjectRef,
                    value,
                },
            };
        }

        // SAFETY: `value.ctx` is the live JavaScriptCore context associated
        // with `value`; this creates a non-owning view for error conversion.
        let ctx = unsafe { JSContext::borrowed(value.ctx) };
        let message = match value.as_string() {
            Ok(message) => message.to_string(),
            Err(error) => return error,
        };

        Self::from_message(&ctx, message)
    }

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
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `ctx.inner` is a live context and `arguments` points to
            // `argument_count` raw JS values for the duration of this call.
            unsafe {
                JSObjectMakeError(ctx.inner, argument_count, arguments, &mut exception)
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(Self::fallback(
                ctx,
                "Error",
                "failed to create JavaScript Error object",
            ));
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

        // SAFETY: `ctx.inner` is live and the temporary message string owns a
        // live `JSStringRef` for the duration of the call. JavaScriptCore
        // initializes `exception` if construction throws.
        let result = unsafe {
            JSObjectMakeTypeError(ctx.inner, message.into().inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(Self::fallback(
                ctx,
                "TypeError",
                "failed to create JavaScript TypeError object",
            ));
        }

        Ok(Self::from(JSObject::from_ref(result, ctx.inner)))
    }

    pub fn new_typ_raw(ctx: &JSContext, message: impl Into<JSString>) -> JSValueRef {
        match Self::new_typ(ctx, message) {
            Ok(error) | Err(error) => error.object.value.inner,
        }
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
        match self.message() {
            Ok(message) => write!(f, "JavaScript error: {:?}", message),
            Err(_) => write!(f, "JavaScript error"),
        }
    }
}

impl std::error::Error for JSError {}

impl From<JSValue> for JSError {
    fn from(value: JSValue) -> Self {
        Self::from_exception(value)
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
        assert!(result.unwrap().as_boolean());
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
        assert!(result.unwrap().as_boolean());
    }

    #[test]
    fn test_error_from_primitive_exception() {
        let ctx = JSContext::new();
        let error = ctx.evaluate_script("throw 42", None).unwrap_err();

        assert_eq!(error.name().unwrap().to_string(), "Error");
        assert_eq!(error.message().unwrap().to_string(), "42");
    }
}
