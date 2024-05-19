use rust_jsc_sys::{JSObjectMakeRegExp, JSValueRef};

use crate::{JSContext, JSError, JSObject, JSRegExp, JSResult, JSValue};

impl JSRegExp {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    /// Creates a new `JSRegExp` object.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the regexp in.
    /// - `args`: The values to initialize the regexp with.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSRegExp, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let regexp = JSRegExp::new_regexp(&ctx, &[JSValue::string(&ctx, "a")]).unwrap();
    /// let result = regexp.exec(&ctx, "abc").unwrap();
    /// assert_eq!(result.as_string().unwrap(), "a");
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the regexp.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The new `JSRegExp` object.
    pub fn new_regexp(ctx: &JSContext, args: &[JSValue]) -> JSResult<Self> {
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

    /// Executes a search for a match in a specified string.
    /// Returns the first match, or `null` if no match was found.
    /// This is equivalent to `regexp.exec(string)` in JavaScript.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to execute the search in.
    /// - `string`: The string to search for a match in.
    ///
    /// # Returns
    /// The first match, or `null` if no match was found.
    ///
    /// # Errors
    /// If an exception is thrown while executing the search.
    /// A `JSError` will be returned.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSRegExp, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let regexp = JSRegExp::new_regexp(&ctx, &[JSValue::string(&ctx, "a")]).unwrap();
    /// let result = regexp.exec(&ctx, "abc").unwrap();
    /// assert_eq!(result.as_string().unwrap(), "a");
    /// ```
    pub fn exec(&self, ctx: &JSContext, string: &str) -> JSResult<JSValue> {
        let string = JSValue::string(ctx, string);
        self.object
            .get_property("exec")?
            .as_object()?
            .call(Some(&self.object), &[string])
    }

    /// Tests for a match in a specified string.
    /// Returns `true` if a match was found, otherwise `false`.
    /// This is equivalent to `regexp.test(string)` in JavaScript.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to execute the test in.
    /// - `string`: The string to test for a match in.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSRegExp, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let regexp = JSRegExp::new_regexp(&ctx, &[JSValue::string(&ctx, "a")]).unwrap();
    /// let result = regexp.test(&ctx, "abc").unwrap();
    /// assert_eq!(result.as_boolean(), true);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while executing the test.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// `true` if a match was found, otherwise `false`.
    pub fn test(&self, ctx: &JSContext, string: &str) -> JSResult<JSValue> {
        let string = JSValue::string(ctx, string);
        self.object
            .get_property("test")?
            .as_object()?
            .call(Some(&self.object), &[string])
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

#[cfg(test)]
mod tests {
    use crate::{JSContext, JSRegExp, JSValue};

    #[test]
    fn test_regexp() {
        let ctx = JSContext::new();
        let regexp = JSRegExp::new_regexp(&ctx, &[JSValue::string(&ctx, "a")]).unwrap();
        let result = regexp.exec(&ctx, "abc").unwrap();
        assert_eq!(result.as_string().unwrap(), "a");

        let result = regexp.test(&ctx, "abc").unwrap();
        assert_eq!(result.as_boolean(), true);
    }
}
