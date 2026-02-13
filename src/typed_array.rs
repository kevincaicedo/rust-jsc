use rust_jsc_sys::{
    JSObjectGetArrayBufferByteLength, JSObjectGetArrayBufferBytesPtr,
    JSObjectGetTypedArrayBuffer, JSObjectGetTypedArrayByteLength,
    JSObjectGetTypedArrayByteOffset, JSObjectGetTypedArrayBytesPtr,
    JSObjectGetTypedArrayLength, JSObjectIsDetachedBuffer,
    JSObjectMakeArrayBufferWithBytesNoCopy, JSObjectMakeTypedArray,
    JSObjectMakeTypedArrayWithArrayBuffer,
    JSObjectMakeTypedArrayWithArrayBufferAndOffset,
    JSObjectMakeTypedArrayWithBytesNoCopy, JSValueGetTypedArrayBytesPtrFromValue,
    JSValueGetTypedArrayType, JSValueRef,
};

use crate::{
    JSArrayBuffer, JSContext, JSError, JSObject, JSResult, JSTypedArray,
    JSTypedArrayType, JSValue,
};

impl JSTypedArray {
    /// Creates a JavaScript Typed Array object with the given number of elements.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the typed array in.
    /// - `length`: The number of elements in the typed array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the typed array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    ///  A JSTypedArray that is a Typed Array with all elements set to zero.
    pub fn new(ctx: &JSContext, length: usize) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeTypedArray(
                ctx.inner,
                JSTypedArrayType::Uint8Array as _,
                length,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        let object = JSObject::from_ref(result, ctx.inner);
        Ok(Self { object })
    }

    /// Creates a JSTypedArray from a given JSValue.
    pub fn from_value(value: &JSValue) -> JSResult<Self> {
        let object = value.as_object()?;
        Ok(Self { object })
    }

    /// Creates a JavaScript Typed Array object from an existing pointer.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the typed array in.
    /// - `bytes`: The pointer to the data to use for the typed array.
    /// - `array_type`: The type of the typed array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let mut bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_mut_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the typed array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// A JSTypedArray that is a Typed Array with the given data.
    pub fn with_bytes<T>(
        ctx: &JSContext,
        bytes: &mut [T],
        array_type: JSTypedArrayType,
    ) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeTypedArrayWithBytesNoCopy(
                ctx.inner,
                array_type as _,
                bytes.as_mut_ptr() as _,
                bytes.len() as _,
                None,
                std::ptr::null_mut(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(
                JSError::with_message(ctx, "Failed to create typed array").unwrap()
            );
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }

    /// Gets the type of the Typed Array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.array_type().unwrap(), JSTypedArrayType::Uint8Array);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the type.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The type of the Typed Array.
    pub fn array_type(&self) -> JSResult<JSTypedArrayType> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let _type = unsafe {
            JSValueGetTypedArrayType(
                self.object.ctx,
                self.object.value.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(JSTypedArrayType::from_type(_type))
    }

    /// Gets the length of the Typed Array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the length.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The length of the Typed Array.
    pub fn len(&self) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetTypedArrayLength(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Gets the length of the Typed Array in bytes.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.byte_len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the length.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The byte length of the Typed Array object or 0 if the object is not a Typed Array object.
    pub fn byte_len(&self) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetTypedArrayByteLength(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Gets the byte offset of the Typed Array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.byte_offset().unwrap(), 0);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the byte offset.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The byte offset of the Typed Array object or 0 if the object is not a Typed Array object.
    pub fn byte_offset(&self) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetTypedArrayByteOffset(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Gets the buffer of the Typed Array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray};
    ///
    /// let ctx = JSContext::new();
    /// let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
    /// assert_eq!(typed_array.get_buffer().unwrap().len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the buffer.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The buffer of the Typed Array object or `null` if the object is not a Typed Array object.
    pub fn get_buffer(&self) -> JSResult<JSArrayBuffer> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetTypedArrayBuffer(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(JSArrayBuffer::from_object(JSObject::from_ref(
            result,
            self.object.ctx,
        )))
    }

    /// Gets the bytes of the Typed Array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let mut bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_mut_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// assert_eq!(typed_array.len().unwrap(), 5);
    /// assert_eq!(typed_array.byte_len().unwrap(), 5);
    /// assert_eq!(typed_array.byte_offset().unwrap(), 0);
    /// assert_eq!(typed_array.bytes::<u8>().unwrap().len(), 5);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the Typed Array object or `null` if the object is not a Typed Array object.
    pub fn bytes<T>(&self) -> JSResult<&mut [T]> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetTypedArrayBytesPtr(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let context = JSContext::from(self.object.ctx);
            return Err(JSError::with_message(
                &context,
                "Typed array bytes pointer is null",
            )?);
        }

        let byte_offset = self.byte_offset()?;
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                // result as *mut u8,
                result.offset(byte_offset as isize).cast::<T>(),
                self.byte_len()?,
            )
        };

        Ok(bytes)
    }

    pub fn bytes_from_value<T>(value: &JSValue) -> JSResult<&mut [T]> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut offset: usize = 0;
        let mut len: usize = 0;

        let result = unsafe {
            JSValueGetTypedArrayBytesPtrFromValue(
                value.ctx,
                value.inner,
                &mut exception,
                &mut offset,
                &mut len,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let context = JSContext::from(value.ctx);
            return Err(JSError::with_message(
                &context,
                "Typed array bytes pointer is null",
            )?);
        }

        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                result.offset(offset as isize).cast::<T>(),
                len,
            )
        };

        Ok(bytes)
    }

    /// Gets the bytes of the Typed Array as a Vec.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let mut bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_mut_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the Typed Array object as a Vec or `null` if the object is not a Typed Array object.
    pub fn as_vec<T: Clone>(&self) -> JSResult<Vec<T>> {
        Ok(self.bytes::<T>()?.to_vec())
    }

    /// Creates a JavaScript Typed Array object from an existing buffer.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the typed array in.
    /// - `array_buffer`: The buffer to use for the typed array.
    /// - `array_type`: The type of the typed array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// let typed_array = JSTypedArray::with_buffer(&ctx, array_buffer, JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the typed array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// A JSTypedArray that is a Typed Array with the given buffer.
    pub fn with_buffer(
        ctx: &JSContext,
        array_buffer: JSArrayBuffer,
        array_type: JSTypedArrayType,
    ) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectMakeTypedArrayWithArrayBuffer(
                ctx.inner,
                array_type as _,
                array_buffer.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }

    /// Creates a JavaScript Typed Array object from an existing buffer with an offset.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the typed array in.
    /// - `array_buffer`: The buffer to use for the typed array.
    /// - `array_type`: The type of the typed array.
    /// - `byte_offset`: The offset in bytes to start the typed array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// let typed_array = JSTypedArray::with_buffer_and_offset(&ctx, array_buffer, JSTypedArrayType::Uint8Array, 2).unwrap();
    /// assert_eq!(typed_array.len().unwrap(), 8);
    /// assert_eq!(typed_array.byte_len().unwrap(), 8);
    /// assert_eq!(typed_array.byte_offset().unwrap(), 2);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the typed array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// A JSTypedArray that is a Typed Array with the given buffer and offset.
    pub fn with_buffer_and_offset(
        ctx: &JSContext,
        array_buffer: JSArrayBuffer,
        array_type: JSTypedArrayType,
        byte_offset: usize,
    ) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let byte_length = array_buffer.len()? - byte_offset;
        let result = unsafe {
            JSObjectMakeTypedArrayWithArrayBufferAndOffset(
                ctx.inner,
                array_type as _,
                array_buffer.object.inner,
                byte_offset as _,
                byte_length as _,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }
}

impl From<JSTypedArray> for JSObject {
    fn from(typed_array: JSTypedArray) -> Self {
        typed_array.object
    }
}

impl From<JSTypedArray> for JSValue {
    fn from(typed_array: JSTypedArray) -> Self {
        typed_array.object.into()
    }
}

impl From<JSObject> for JSTypedArray {
    fn from(object: JSObject) -> Self {
        Self { object }
    }
}

impl JSArrayBuffer {
    /// Creates a new `JSArrayBuffer` object from a given JSObject.
    pub fn from_object(object: JSObject) -> Self {
        Self { object }
    }

    /// Gets the length of the ArrayBuffer.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// assert_eq!(array_buffer.len().unwrap(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the length.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The length of the ArrayBuffer object or 0 if the object is not an ArrayBuffer object.
    pub fn len(&self) -> JSResult<usize> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetArrayBufferByteLength(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Gets the bytes of the ArrayBuffer.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// assert_eq!(array_buffer.bytes().unwrap().len(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the ArrayBuffer object or `null` if the object is not an ArrayBuffer object.
    pub fn bytes(&self) -> JSResult<&mut [u8]> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectGetArrayBufferBytesPtr(
                self.object.ctx,
                self.object.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }

        assert!(!result.is_null(), "ArrayBuffer pointer is null");

        let bytes =
            unsafe { std::slice::from_raw_parts_mut(result as *mut u8, self.len()?) };

        Ok(bytes)
    }

    /// Checks if the ArrayBuffer is detached.
    /// Detached ArrayBuffers are ArrayBuffers that have been detached from their backing store.
    /// This can happen when the backing store is transferred to another object.
    pub fn is_detached(&self) -> bool {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSObjectIsDetachedBuffer(self.object.ctx, self.object.inner, &mut exception)
        };
        // TODO: Handle exception
        result
    }

    /// Gets the bytes of the ArrayBuffer as a Vec.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// assert_eq!(array_buffer.as_vec().unwrap().len(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the ArrayBuffer object as a Vec or `null` if the object is not an ArrayBuffer object.
    pub fn new(ctx: &JSContext, bytes: &mut [u8]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.inner,
                bytes.as_mut_ptr() as _,
                bytes.len() as _,
                None,
                std::ptr::null_mut(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(
                JSError::with_message(ctx, "Failed to create array array").unwrap()
            );
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }

    /// Gets the bytes of the ArrayBuffer as a Vec.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// assert_eq!(array_buffer.as_vec().unwrap().len(), 10);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the ArrayBuffer object as a Vec or `null` if the object is not an ArrayBuffer object.
    pub fn as_vec(&self) -> JSResult<Vec<u8>> {
        Ok(self.bytes()?.to_vec())
    }
}

impl From<JSArrayBuffer> for JSObject {
    fn from(array_buffer: JSArrayBuffer) -> Self {
        array_buffer.object
    }
}

impl From<JSArrayBuffer> for JSValue {
    fn from(array_buffer: JSArrayBuffer) -> Self {
        array_buffer.object.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{JSArrayBuffer, JSContext, JSTypedArray, JSTypedArrayType};

    #[test]
    fn test_typed_array_check_type() {
        let ctx = JSContext::new();
        let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint8Array
        );
    }

    #[test]
    fn test_typed_array() {
        let ctx = JSContext::new();
        let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint8Array
        );
        assert_eq!(typed_array.len().unwrap(), 10);
        assert_eq!(typed_array.byte_len().unwrap(), 10);
        assert_eq!(typed_array.byte_offset().unwrap(), 0);
        assert_eq!(typed_array.get_buffer().unwrap().len().unwrap(), 10);
    }

    #[test]
    fn test_typed_array_with_bytes() {
        let ctx = JSContext::new();
        let mut bytes = vec![6, 5, 5, 6, 9];
        let typed_array = JSTypedArray::with_bytes::<u8>(
            &ctx,
            bytes.as_mut_slice(),
            JSTypedArrayType::Uint8Array,
        )
        .unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint8Array
        );

        assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
        assert_eq!(typed_array.len().unwrap(), 5);
        assert_eq!(typed_array.byte_len().unwrap(), 5);
        assert_eq!(typed_array.byte_offset().unwrap(), 0);

        ctx.global_object()
            .set_property(
                "custom_array",
                &typed_array.clone().into(),
                Default::default(),
            )
            .unwrap();

        let result = ctx
            .evaluate_script("new Uint8Array(custom_array.buffer, 1, 3)", None)
            .unwrap();

        let typed_array = JSTypedArray::from_value(&result).unwrap();
        assert_eq!(typed_array.len().unwrap(), 3);
        assert_eq!(typed_array.byte_len().unwrap(), 3);
        assert_eq!(typed_array.byte_offset().unwrap(), 1);

        let result = typed_array.bytes::<u8>().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result, &[5, 5, 6,]);
    }

    #[test]
    fn test_typed_array_with_bytes_offset() {
        let ctx = JSContext::new();
        let mut bytes = vec![6; 10];
        let typed_array = JSTypedArray::with_bytes::<u16>(
            &ctx,
            bytes.as_mut_slice(),
            JSTypedArrayType::Uint16Array,
        )
        .unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint16Array
        );
        assert_eq!(typed_array.len().unwrap(), 5);
        assert_eq!(typed_array.byte_len().unwrap(), 10);
        assert_eq!(typed_array.byte_offset().unwrap(), 0);
        assert_eq!(typed_array.bytes::<u16>().unwrap().len(), 10);
        assert_eq!(typed_array.as_vec::<u16>().unwrap(), vec![6; 10]);

        ctx.global_object()
            .set_property(
                "custom_array",
                &typed_array.clone().into(),
                Default::default(),
            )
            .unwrap();
        let result = ctx
            .evaluate_script("new Uint16Array(custom_array.buffer, 2, 4)", None)
            .unwrap();

        let typed_array = JSTypedArray::from_value(&result).unwrap();
        assert_eq!(typed_array.len().unwrap(), 4);
        assert_eq!(typed_array.byte_len().unwrap(), 8);
        assert_eq!(typed_array.byte_offset().unwrap(), 2);
    }

    #[test]
    fn test_typed_array_len() {
        let ctx = JSContext::new();
        let typed_array = JSTypedArray::new(&ctx, 10).unwrap();
        assert_eq!(typed_array.len().unwrap(), 10);
        assert_eq!(typed_array.byte_len().unwrap(), 10);
    }

    #[test]
    fn test_typed_array_check_byte_data() {
        let ctx = JSContext::new();
        let array = ctx
            .evaluate_script("const array = new Uint8Array([5, 4, 4, 5]); array", None)
            .unwrap();
        let array = JSTypedArray::from_value(&array).unwrap();

        assert_eq!(array.array_type().unwrap(), JSTypedArrayType::Uint8Array);
        assert_eq!(array.len().unwrap(), 4);
        assert_eq!(array.byte_offset().unwrap(), 0);
        assert_eq!(array.byte_len().unwrap(), 4);
        assert_eq!(array.as_vec::<u8>().unwrap(), &[5, 4, 4, 5]);
    }

    #[test]
    fn test_typed_array_with_buffer() {
        let ctx = JSContext::new();
        let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
        let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
        let typed_array =
            JSTypedArray::with_buffer(&ctx, array_buffer, JSTypedArrayType::Uint8Array)
                .unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint8Array
        );
        assert_eq!(typed_array.len().unwrap(), 10);
        assert_eq!(typed_array.byte_len().unwrap(), 10);
        assert_eq!(typed_array.byte_offset().unwrap(), 0);
        assert_eq!(typed_array.get_buffer().unwrap().len().unwrap(), 10);
    }

    #[test]
    fn test_typed_array_with_buffer_and_offset() {
        let ctx = JSContext::new();
        let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
        let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
        let typed_array = JSTypedArray::with_buffer_and_offset(
            &ctx,
            array_buffer,
            JSTypedArrayType::Uint8Array,
            2,
        )
        .unwrap();
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint8Array
        );
        assert_eq!(typed_array.len().unwrap(), 8);
        assert_eq!(typed_array.byte_len().unwrap(), 8);
        assert_eq!(typed_array.byte_offset().unwrap(), 2);
        assert_eq!(typed_array.get_buffer().unwrap().len().unwrap(), 10);
    }

    #[test]
    fn test_array_buffer() {
        let ctx = JSContext::new();
        let mut bytes = vec![6; 10];
        let array_buffer = JSArrayBuffer::new(&ctx, bytes.as_mut_slice()).unwrap();
        assert_eq!(array_buffer.len().unwrap(), 10);
        assert_eq!(array_buffer.as_vec().unwrap(), vec![6; 10]);
    }

    #[test]
    fn test_array_buffer_with_bytes() {
        let ctx = JSContext::new();
        let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
        let array_buffer = JSTypedArray::from_value(&array_buffer).unwrap();

        assert_eq!(
            array_buffer.array_type().unwrap(),
            JSTypedArrayType::ArrayBuffer
        );
    }

    #[test]
    fn test_array_buffer_is_detached() {
        let ctx = JSContext::new();
        let mut bytes = vec![6; 10];
        let array_buffer = JSArrayBuffer::new(&ctx, bytes.as_mut_slice()).unwrap();
        assert_eq!(array_buffer.is_detached(), false);

        let array_buffer = ctx
            .evaluate_script("const buffer = new ArrayBuffer(10); buffer", None)
            .unwrap();
        let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
        let _result = ctx
            .evaluate_script(
                "var sample = new DataView(buffer, 0); var dest = buffer.transfer(5);",
                None,
            )
            .unwrap();
        assert_eq!(array_buffer.is_detached(), true);
    }
}
