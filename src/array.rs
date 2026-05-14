use std::ops::Deref;

use rust_jsc_sys::{
    JSObjectArrayPush, JSObjectGetArrayLength, JSObjectMakeArray, JSValueRef,
};

use crate::{
    with_raw_value_refs, JSArray, JSContext, JSError, JSObject, JSResult, JSValue,
};

impl JSArray {
    pub fn new(object: JSObject) -> Self {
        Self { object }
    }

    /// Creates a new `JSArray` object.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the array in.
    /// - `args`: The values to initialize the array with.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSArray, JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let array = JSArray::new_array(
    ///     &ctx,
    ///     &[
    ///         JSValue::number(&ctx, 1.0),
    ///         JSValue::number(&ctx, 2.0),
    ///         JSValue::number(&ctx, 3.0),
    ///      ]
    /// ).unwrap();
    /// assert_eq!(array.as_string().unwrap(), "1,2,3");
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The new `JSArray` object.
    pub fn new_array(ctx: &JSContext, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `ctx.inner` is a live context and `arguments` points to
            // `argument_count` raw JS values for the duration of this call.
            unsafe {
                JSObjectMakeArray(ctx.inner, argument_count, arguments, &mut exception)
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::from_message(
                ctx,
                "failed to create JavaScript array",
            ));
        }

        Ok(Self::new(JSObject::from_ref(result, ctx.inner)))
    }

    /// Gets the value at the specified index.
    /// This is equivalent to `array[index]` in JavaScript.
    ///
    /// # Arguments
    /// - `index`: The index of the value to get.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSArray, JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let array = JSArray::new_array(
    ///    &ctx,
    ///    &[
    ///      JSValue::number(&ctx, 1.0),
    ///      JSValue::number(&ctx, 2.0),
    ///      JSValue::number(&ctx, 3.0),
    ///    ]
    /// ).unwrap();
    /// assert_eq!(array.get(0).unwrap().as_number().unwrap(), 1.0);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the value.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The value at the specified index.
    pub fn get(&self, index: u32) -> JSResult<JSValue> {
        self.object.get_property_at_index(index)
    }

    /// Sets the value at the specified index.
    /// This is equivalent to `array[index] = value` in JavaScript.
    ///
    /// # Arguments
    /// - `index`: The index of the value to set.
    /// - `value`: The value to set.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSArray, JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let array = JSArray::new_array(
    ///    &ctx,
    ///    &[
    ///       JSValue::number(&ctx, 1.0),
    ///       JSValue::number(&ctx, 2.0),
    ///       JSValue::number(&ctx, 3.0),
    ///     ]
    /// ).unwrap();
    /// array.set(0, &JSValue::number(&ctx, 4.0)).unwrap();
    /// array.set(1, &JSValue::number(&ctx, 5.0)).unwrap();
    /// array.set(2, &JSValue::number(&ctx, 6.0)).unwrap();
    /// assert_eq!(array.as_string().unwrap(), "4,5,6");
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while setting the value.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// An empty `JSResult`.
    pub fn set(&self, index: u32, value: &JSValue) -> JSResult<()> {
        self.object.set_property_at_index(index, value)
    }

    /// Gets the length of an exact JavaScript Array.
    ///
    /// This uses JavaScriptCore's native array length instead of reading the
    /// observable `array.length` property. Proxies and array-like objects are
    /// rejected.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSArray, JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let array = JSArray::new_array(
    ///    &ctx,
    ///    &[
    ///       JSValue::number(&ctx, 1.0),
    ///       JSValue::number(&ctx, 2.0),
    ///       JSValue::number(&ctx, 3.0),
    ///    ]
    /// ).unwrap();
    /// assert_eq!(array.length().unwrap(), 3);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the length.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The length of the array.
    pub fn length(&self) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut length = 0;
        // SAFETY: `self.object` holds a live object/context pair. The native
        // helper writes either `length` or `exception` without taking ownership.
        let ok = unsafe {
            JSObjectGetArrayLength(
                self.object.ctx,
                self.object.inner,
                &mut length,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        if !ok {
            // SAFETY: this creates a borrowed, non-releasing context view from
            // the live context pointer already stored on `self.object`.
            let ctx = unsafe { JSContext::borrowed(self.object.ctx) };
            return Err(JSError::from_message(
                &ctx,
                "failed to get JavaScript array length",
            ));
        }

        Ok(length)
    }

    /// Pushes a value to the end of the array.
    /// This is equivalent to `array.push(value)` in JavaScript.
    /// Returns the new length of the array.
    ///
    /// # Arguments
    /// - `value`: The value to push.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSArray, JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let array = JSArray::new_array(
    ///    &ctx,
    ///    &[
    ///       JSValue::number(&ctx, 1.0),
    ///       JSValue::number(&ctx, 2.0),
    ///       JSValue::number(&ctx, 3.0),
    ///    ]
    /// ).unwrap();
    /// array.push(&JSValue::number(&ctx, 4 as f64)).unwrap();
    /// array.push(&JSValue::number(&ctx, 5 as f64)).unwrap();
    /// array.push(&JSValue::number(&ctx, 6 as f64)).unwrap();
    /// assert_eq!(array.as_string().unwrap(), "1,2,3,4,5,6");
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while pushing the value.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The new length of the array.
    pub fn push(&self, value: &JSValue) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut new_length = 0;
        // SAFETY: `self.object` and `value` hold live same-context
        // JavaScriptCore handles. The helper writes either `new_length` or
        // `exception` without taking ownership.
        let ok = unsafe {
            JSObjectArrayPush(
                self.object.ctx,
                self.object.inner,
                value.inner,
                &mut new_length,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        if !ok {
            // SAFETY: this creates a borrowed, non-releasing context view from
            // the live context pointer already stored on `self.object`.
            let ctx = unsafe { JSContext::borrowed(self.object.ctx) };
            return Err(JSError::from_message(
                &ctx,
                "failed to push JavaScript array value",
            ));
        }

        Ok(new_length)
    }
}

impl Deref for JSArray {
    type Target = JSValue;

    fn deref(&self) -> &JSValue {
        &self.object
    }
}

impl From<JSArray> for JSObject {
    fn from(array: JSArray) -> Self {
        array.object
    }
}

impl From<JSArray> for JSValue {
    fn from(array: JSArray) -> Self {
        array.object.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{JSArray, JSContext, JSValue};

    #[test]
    fn test_array() {
        let ctx = JSContext::new();
        let array = JSArray::new_array(
            &ctx,
            &[
                JSValue::number(&ctx, 1.0),
                JSValue::number(&ctx, 2.0),
                JSValue::number(&ctx, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(array.as_string().unwrap(), "1,2,3");
    }

    #[test]
    fn test_array_get() {
        let ctx = JSContext::new();
        let array = JSArray::new_array(
            &ctx,
            &[
                JSValue::number(&ctx, 1.0),
                JSValue::number(&ctx, 2.0),
                JSValue::number(&ctx, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(array.get(0).unwrap().as_number().unwrap(), 1.0);
        assert_eq!(array.get(1).unwrap().as_number().unwrap(), 2.0);
        assert_eq!(array.get(2).unwrap().as_number().unwrap(), 3.0);
    }

    #[test]
    fn test_array_set() {
        let ctx = JSContext::new();
        let array = JSArray::new_array(
            &ctx,
            &[
                JSValue::number(&ctx, 1.0),
                JSValue::number(&ctx, 2.0),
                JSValue::number(&ctx, 3.0),
            ],
        )
        .unwrap();
        array.set(0, &JSValue::number(&ctx, 4.0)).unwrap();
        array.set(1, &JSValue::number(&ctx, 5.0)).unwrap();
        array.set(2, &JSValue::number(&ctx, 6.0)).unwrap();
        assert_eq!(array.as_string().unwrap(), "4,5,6");
    }

    #[test]
    fn test_array_length() {
        let ctx = JSContext::new();
        let array = JSArray::new_array(
            &ctx,
            &[
                JSValue::number(&ctx, 1.0),
                JSValue::number(&ctx, 2.0),
                JSValue::number(&ctx, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(array.length().unwrap(), 3);
    }

    #[test]
    fn test_array_length_rejects_proxy() {
        let ctx = JSContext::new();
        let array = ctx
            .evaluate_script(
                "new Proxy([], { get(_target, property) { if (property === 'length') throw new Error('length failed'); return 0; } })",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();
        let array = JSArray::new(array);

        let error = array.length().unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "JSObjectGetArrayLength expects object to be an Array object"
        );
    }

    #[test]
    fn test_array_set_propagates_exception() {
        let ctx = JSContext::new();
        let array = ctx
            .evaluate_script(
                "new Proxy([], { set() { throw new Error('set failed'); } })",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();
        let array = JSArray::new(array);
        let value = JSValue::number(&ctx, 1.0);

        let error = array.set(0, &value).unwrap_err();
        assert_eq!(error.message().unwrap().to_string(), "set failed");
    }

    #[test]
    fn test_array_push() {
        let ctx = JSContext::new();
        let array = JSArray::new_array(
            &ctx,
            &[
                JSValue::number(&ctx, 1.0),
                JSValue::number(&ctx, 2.0),
                JSValue::number(&ctx, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(array.push(&JSValue::number(&ctx, 4.0)).unwrap(), 4);
        assert_eq!(array.push(&JSValue::number(&ctx, 5.0)).unwrap(), 5);
        assert_eq!(array.push(&JSValue::number(&ctx, 6.0)).unwrap(), 6);
        assert_eq!(array.as_string().unwrap(), "1,2,3,4,5,6");
    }
}
