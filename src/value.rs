use rust_jsc_sys::{
    JSContextRef, JSObjectRef, JSValueCreateJSONString, JSValueGetType, JSValueIsArray,
    JSValueIsBoolean, JSValueIsDate, JSValueIsEqual, JSValueIsInstanceOfConstructor,
    JSValueIsNull, JSValueIsNumber, JSValueIsObject, JSValueIsObjectOfClass,
    JSValueIsStrictEqual, JSValueIsString, JSValueIsSymbol, JSValueIsUndefined,
    JSValueMakeBoolean, JSValueMakeFromJSONString, JSValueMakeNull, JSValueMakeNumber,
    JSValueMakeString, JSValueMakeSymbol, JSValueMakeUndefined, JSValueProtect,
    JSValueRef, JSValueToBoolean, JSValueToNumber, JSValueToObject, JSValueToStringCopy,
    JSValueUnprotect,
};

use crate::{
    JSClass, JSContext, JSError, JSObject, JSResult, JSString, JSValue, JSValueType,
};

impl JSValue {
    /// Creates a new `JSValue` object.
    pub fn new(inner: JSValueRef, ctx: JSContextRef) -> Self {
        Self { inner, ctx }
    }

    /// Creates a JavaScript boolean value.
    ///
    /// # Arguments
    /// * `value` - The boolean value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::boolean(&ctx, true);
    /// assert!(value.is_boolean());
    /// ```
    ///
    /// # Returns
    /// A JavaScript boolean value.
    pub fn boolean(ctx: &JSContext, value: bool) -> JSValue {
        let inner = unsafe { JSValueMakeBoolean(ctx.inner, value) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript undefined value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::undefined(&ctx);
    /// assert!(value.is_undefined());
    /// ```
    ///
    /// # Returns
    /// A JavaScript undefined value.
    pub fn undefined(ctx: &JSContext) -> JSValue {
        let inner = unsafe { JSValueMakeUndefined(ctx.inner) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript null value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::null(&ctx);
    /// assert!(value.is_null());
    /// ```
    ///
    /// # Returns
    /// A JavaScript null value.
    pub fn null(ctx: &JSContext) -> JSValue {
        let inner = unsafe { JSValueMakeNull(ctx.inner) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript number value from a double-precision floating-point number.
    ///
    /// # Arguments
    /// * `value` - The number to use as the value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// assert!(value.is_number());
    /// ```
    ///
    /// # Returns
    /// A JavaScript number value.
    pub fn number(ctx: &JSContext, value: f64) -> JSValue {
        let inner = unsafe { JSValueMakeNumber(ctx.inner, value) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript string value from a string.
    ///
    /// # Arguments
    /// * `value` - The string to use as the value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::string(&ctx, "Hello, World!");
    /// assert!(value.is_string());
    /// ```
    ///
    /// # Returns
    /// A JavaScript string value.
    pub fn string(ctx: &JSContext, value: impl Into<JSString>) -> JSValue {
        let inner = unsafe { JSValueMakeString(ctx.inner, value.into().inner) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript symbol value.
    ///
    /// # Arguments
    /// * `description` - A description of the symbol.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::symbol(&ctx, "symbol");
    /// assert!(value.is_symbol());
    /// ```
    ///
    /// # Returns
    /// A JavaScript symbol value.
    pub fn symbol(ctx: &JSContext, description: impl Into<JSString>) -> JSValue {
        let inner = unsafe { JSValueMakeSymbol(ctx.inner, description.into().inner) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript value from a JSON serialized string.
    ///
    /// # Arguments
    /// * `string` - The JSON serialized string.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::from_json(&ctx, r#"{"key": "value"}"#);
    /// assert!(value.is_object());
    /// ```
    ///
    /// # Returns
    /// A JavaScript value, or null if the input is not valid JSON.
    pub fn from_json(ctx: &JSContext, string: impl Into<JSString>) -> JSValue {
        let string = string.into();
        let inner = unsafe { JSValueMakeFromJSONString(ctx.inner, string.inner) };
        Self::new(inner, ctx.inner)
    }

    /// Creates a JavaScript string containing the JSON serialized representation of a JS value.
    ///
    /// # Arguments
    /// * `indent` - The number of spaces to indent when nesting.  If 0, the resulting JSON will not contains newlines. The size of the indent is clamped to 10 spaces.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = ctx.evaluate_script("({ key: 'value' })", None).unwrap();
    /// assert!(value.as_json_string(2).is_ok());
    /// ```
    ///
    /// # Returns
    /// A JSString with the result of serialization, or JSError if an exception occurs.
    pub fn as_json_string(&self, indent: u32) -> JSResult<JSString> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let string = unsafe {
            JSValueCreateJSONString(self.ctx, self.inner, indent, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(string.into())
    }

    /// Converts a JavaScript value to a js string and returns the resulting js string.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::string(&ctx, "Hello, World!");
    /// assert!(value.as_string().is_ok());
    /// ```
    ///
    /// # Returns
    /// A JavaScript string.
    pub fn as_string(&self) -> JSResult<JSString> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let string = unsafe { JSValueToStringCopy(self.ctx, self.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(string.into())
    }

    /// Converts a JavaScript value to an object and returns the resulting object.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = ctx.evaluate_script("({ key: 'value' })", None).unwrap();
    /// assert!(value.as_object().is_ok());
    /// ```
    ///
    /// # Returns
    /// A JavaScript object.
    pub fn as_object(&self) -> JSResult<JSObject> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let object = unsafe { JSValueToObject(self.ctx, self.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(JSObject::from_ref(object, self.ctx))
    }

    /// Converts a JavaScript value to boolean and returns the resulting boolean.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::boolean(&ctx, true);
    /// assert_eq!(value.as_boolean(), true);
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn as_boolean(&self) -> bool {
        unsafe { JSValueToBoolean(self.ctx, self.inner) }
    }

    /// Converts a JavaScript value to number and returns the resulting number.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// assert_eq!(value.as_number().unwrap(), 42.0);
    /// ```
    ///
    /// # Returns
    /// A number value.
    pub fn as_number(&self) -> JSResult<f64> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let number = unsafe { JSValueToNumber(self.ctx, self.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(number)
    }

    /// Checks if the value is undefined.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::undefined(&ctx);
    /// assert!(value.is_undefined());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_undefined(&self) -> bool {
        unsafe { JSValueIsUndefined(self.ctx, self.inner) }
    }

    /// Checks if the value is null.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::null(&ctx);
    /// assert!(value.is_null());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_null(&self) -> bool {
        unsafe { JSValueIsNull(self.ctx, self.inner) }
    }

    /// Checks if the value is a boolean.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::boolean(&ctx, true);
    /// assert!(value.is_boolean());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_boolean(&self) -> bool {
        unsafe { JSValueIsBoolean(self.ctx, self.inner) }
    }

    /// Checks if the value is a number.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// assert!(value.is_number());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_number(&self) -> bool {
        unsafe { JSValueIsNumber(self.ctx, self.inner) }
    }

    /// Checks if the value is a string.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::string(&ctx, "Hello, World!");
    /// assert!(value.is_string());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_string(&self) -> bool {
        unsafe { JSValueIsString(self.ctx, self.inner) }
    }

    /// Checks if the value is an object.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = ctx.evaluate_script("({ key: 'value' })", None).unwrap();
    /// assert!(value.is_object());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_object(&self) -> bool {
        unsafe { JSValueIsObject(self.ctx, self.inner) }
    }

    /// Checks if the value is a symbol.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::symbol(&ctx, "symbol");
    /// assert!(value.is_symbol());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_symbol(&self) -> bool {
        unsafe { JSValueIsSymbol(self.ctx, self.inner) }
    }

    /// Checks if the value is an array.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = ctx.evaluate_script("[1, 2, 3]", None).unwrap();
    /// assert!(value.is_array());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_array(&self) -> bool {
        unsafe { JSValueIsArray(self.ctx, self.inner) }
    }

    /// Checks if the value is a date.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = ctx.evaluate_script("new Date()", None).unwrap();
    /// assert!(value.is_date());
    /// ```
    ///
    /// # Returns
    /// A boolean value.
    pub fn is_date(&self) -> bool {
        unsafe { JSValueIsDate(self.ctx, self.inner) }
    }

    /// Tests whether a JavaScript value is an object constructed by a given constructor,
    /// as compared by the JS instanceof operator.
    ///
    /// # Arguments
    /// * `constructor` - The constructor to test against.
    ///
    /// # Returns
    /// true if value is an object constructed by constructor,
    /// as compared by the JS instanceof operator, otherwise false.
    pub fn is_instance_of(&self, constructor: &JSObject) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSValueIsInstanceOfConstructor(
                self.ctx,
                self.inner,
                constructor.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Tests whether an object is an instance of a particular class.
    ///
    /// # Arguments
    /// * `class` - The class to test against.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let class = JSClass::builder("Test").build().unwrap();
    /// let value = class.object::<i32>(&ctx, Some(Box::new(42)));
    /// assert!(value.is_object_of_class(&class).unwrap());
    /// ```
    ///
    /// # Returns
    /// true if the object is an instance of class, otherwise false.
    pub fn is_object_of_class(&self, class: &JSClass) -> JSResult<bool> {
        return Ok(unsafe { JSValueIsObjectOfClass(self.ctx, self.inner, class.inner) });
    }

    /// Tests whether two JavaScript values are equal, as compared by the JS == operator.
    ///
    /// # Arguments
    /// * `other` - The value to compare.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value1 = JSValue::number(&ctx, 42.0);
    /// let value2 = JSValue::number(&ctx, 42.0);
    /// assert!(value1.is_equal(&value2).unwrap());
    /// ```
    ///
    /// # Returns
    /// true if the values are equal, otherwise false.
    pub fn is_equal(&self, other: &JSValue) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result =
            unsafe { JSValueIsEqual(self.ctx, self.inner, other.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Protects a JavaScript value from garbage collection.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// value.protect();
    /// ```
    ///
    /// # Note
    /// You must call `unprotect` when you no longer need the value.
    /// Use this method when you want to store a JSValue in a global or on the heap,
    /// where the garbage collector will not be able to discover your reference to it.\n
    /// A value may be protected multiple times and must be unprotected an equal number of times
    /// before becoming eligible for garbage collection.
    pub fn protect(&self) {
        unsafe { JSValueProtect(self.ctx, self.inner) };
    }

    /// Unprotects a JavaScript value from garbage collection.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// value.protect();
    /// value.unprotect();
    /// ```
    ///
    /// # Note
    /// A value may be protected multiple times and must be unprotected an\n
    /// equal number of times before becoming eligible for garbage collection.
    pub fn unprotect(&self) {
        unsafe { JSValueUnprotect(self.ctx, self.inner) };
    }

    /// Returns the type of a JavaScript value.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let value = JSValue::number(&ctx, 42.0);
    /// assert_eq!(value.get_type(), JSValueType::Number);
    /// ```
    ///
    /// # Returns
    /// The type of the JavaScript value.
    pub fn get_type(&self) -> JSValueType {
        let type_ = unsafe { JSValueGetType(self.ctx, self.inner) };
        JSValueType::from_js_type(type_)
    }
}

/// This is equivalent to `===` in JavaScript.
impl PartialEq for JSValue {
    fn eq(&self, other: &JSValue) -> bool {
        unsafe { JSValueIsStrictEqual(self.ctx, self.inner, other.inner) }
    }
}

impl From<JSValue> for JSValueRef {
    fn from(val: JSValue) -> Self {
        val.inner
    }
}

impl From<JSValue> for JSObjectRef {
    fn from(value: JSValue) -> Self {
        value.inner as *mut _
    }
}

#[cfg(test)]
mod tests {
    use crate::{JSObject, JSValue};

    #[test]
    fn test_boolean() {
        let ctx = crate::JSContext::new();
        let value = JSValue::boolean(&ctx, true);
        assert!(value.is_boolean());
    }

    #[test]
    fn test_undefined() {
        let ctx = crate::JSContext::new();
        let value = JSValue::undefined(&ctx);
        assert!(value.is_undefined());
    }

    #[test]
    fn test_null() {
        let ctx = crate::JSContext::new();
        let value = JSValue::null(&ctx);
        assert!(value.is_null());
    }

    #[test]
    fn test_number() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        assert!(value.is_number());
    }

    #[test]
    fn test_string() {
        let ctx = crate::JSContext::new();
        let value = JSValue::string(&ctx, "Hello, World!");
        assert!(value.is_string());
    }

    #[test]
    fn test_symbol() {
        let ctx = crate::JSContext::new();
        let value = JSValue::symbol(&ctx, "symbol");
        assert!(value.is_symbol());
    }

    #[test]
    fn test_from_json() {
        let ctx = crate::JSContext::new();
        let value = JSValue::from_json(&ctx, r#"{"key": "value"}"#);
        assert!(value.is_object());
    }

    #[test]
    fn test_as_json_string() {
        let ctx = crate::JSContext::new();
        let value = ctx.evaluate_script("({ key: 'value' })", None).unwrap();
        assert!(value.as_json_string(2).is_ok());
    }

    #[test]
    fn test_as_string() {
        let ctx = crate::JSContext::new();
        let value = JSValue::string(&ctx, "Hello, World!");
        assert!(value.as_string().is_ok());
    }

    #[test]
    fn test_as_object() {
        let ctx = crate::JSContext::new();
        let value: JSValue = JSObject::new(&ctx).into();
        assert!(value.as_object().is_ok());
    }

    #[test]
    fn test_as_boolean() {
        let ctx = crate::JSContext::new();
        let value = JSValue::boolean(&ctx, true);
        assert_eq!(value.as_boolean(), true);
    }

    #[test]
    fn test_as_number() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        assert_eq!(value.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_is_undefined() {
        let ctx = crate::JSContext::new();
        let value = JSValue::undefined(&ctx);
        assert!(value.is_undefined());
    }

    #[test]
    fn test_is_null() {
        let ctx = crate::JSContext::new();
        let value = JSValue::null(&ctx);
        assert!(value.is_null());
    }

    #[test]
    fn test_is_boolean() {
        let ctx = crate::JSContext::new();
        let value = JSValue::boolean(&ctx, true);
        assert!(value.is_boolean());
    }

    #[test]
    fn test_is_number() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        assert!(value.is_number());
    }

    #[test]
    fn test_is_string() {
        let ctx = crate::JSContext::new();
        let value = JSValue::string(&ctx, "Hello, World!");
        assert!(value.is_string());
    }

    #[test]
    fn test_is_object() {
        let ctx = crate::JSContext::new();
        let value: JSValue = ctx.evaluate_script("({ key: 'value' })", None).unwrap();
        assert!(value.is_object());
    }

    #[test]
    fn test_is_symbol() {
        let ctx = crate::JSContext::new();
        let value = JSValue::symbol(&ctx, "symbol");
        assert!(value.is_symbol());
    }

    #[test]
    fn test_is_array() {
        let ctx = crate::JSContext::new();
        let value: JSValue = ctx.evaluate_script("[1, 2, 3]", None).unwrap();
        assert!(value.is_array());
    }

    #[test]
    fn test_is_date() {
        let ctx = crate::JSContext::new();
        let value: JSValue = ctx.evaluate_script("new Date()", None).unwrap();
        assert!(value.is_date());
    }

    #[test]
    fn test_is_object_of_class() {
        let ctx = crate::JSContext::new();
        let class = crate::JSClass::builder("Test").build().unwrap();
        let value = class.object::<i32>(&ctx, Some(Box::new(42)));
        assert!(value.is_object_of_class(&class).unwrap());
    }

    #[test]
    fn test_is_equal() {
        let ctx = crate::JSContext::new();
        let value1 = JSValue::number(&ctx, 42.0);
        let value2 = JSValue::number(&ctx, 42.0);
        assert!(value1.is_equal(&value2).unwrap());
    }

    #[test]
    fn test_protect() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        value.protect();
    }

    #[test]
    fn test_unprotect() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        value.protect();
        value.unprotect();
    }

    #[test]
    fn test_get_type() {
        let ctx = crate::JSContext::new();
        let value = JSValue::number(&ctx, 42.0);
        assert_eq!(value.get_type(), crate::JSValueType::Number);
    }

    #[test]
    fn test_partial_eq() {
        let ctx = crate::JSContext::new();
        let value1 = JSValue::number(&ctx, 42.0);
        let value2 = JSValue::number(&ctx, 42.0);
        assert_eq!(value1, value2);
    }
}
