use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeArray, JSValueRef};

use crate::{JSArray, JSContext, JSError, JSObject, JSResult, JSValue};

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
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();

        let result = unsafe {
            JSObjectMakeArray(ctx.inner, args.len(), args.as_ptr(), &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
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

    /// Gets the length of the array.
    /// This is equivalent to `array.length` in JavaScript.
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
    /// assert_eq!(array.length().unwrap(), 3.0);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the length.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The length of the array.
    pub fn length(&self) -> JSResult<f64> {
        self.object.get_property(&"length".into())?.as_number()
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
    pub fn push(&self, value: &JSValue) -> JSResult<f64> {
        let length = self.length()?;
        self.set(length as u32, value)?;
        Ok(length + 1.0)
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
        assert_eq!(array.length().unwrap(), 3.0);
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
        array.push(&JSValue::number(&ctx, 4 as f64)).unwrap();
        array.push(&JSValue::number(&ctx, 5 as f64)).unwrap();
        array.push(&JSValue::number(&ctx, 6 as f64)).unwrap();
        assert_eq!(array.as_string().unwrap(), "1,2,3,4,5,6");
    }
}
