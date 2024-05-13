use rust_jsc_sys::{
    JSObjectGetArrayBufferByteLength, JSObjectGetArrayBufferBytesPtr,
    JSObjectGetTypedArrayBuffer, JSObjectGetTypedArrayByteLength,
    JSObjectGetTypedArrayByteOffset, JSObjectGetTypedArrayLength, JSObjectMakeTypedArray,
    JSObjectMakeTypedArrayWithBytesNoCopy, JSValueGetTypedArrayType, JSValueRef,
};

use crate::{
    JSContext, JSError, JSObject, JSResult, JSTypedArray, JSTypedArrayType, JSValue,
};

impl JSTypedArray {
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

    pub fn new_with_bytes(ctx: &JSContext, bytes: &mut [u8]) -> JSResult<Self> {
        let deallocator = std::ptr::null_mut();
        let mut exception: JSValueRef = std::ptr::null_mut();

        let result = unsafe {
            JSObjectMakeTypedArrayWithBytesNoCopy(
                ctx.inner,
                JSTypedArrayType::Uint8Array as _,
                bytes.as_ptr() as _,
                bytes.len(),
                None,
                deallocator,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::with_message(
                ctx,
                "Failed to create typed array".into(),
            )
            .unwrap());
        }

        let object = JSObject::from_ref(result, ctx.inner);

        Ok(Self { object })
    }

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

    pub fn array_buffer(&self) -> JSResult<JSObject> {
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

        Ok(JSObject::from_ref(result, self.object.ctx))
    }

    pub fn array_buffer_len(&self) -> JSResult<usize> {
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

    pub fn as_bytes(&self) -> JSResult<&mut [u8]> {
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

        let byte_offset = self.byte_offset()?;
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                result.offset(byte_offset as isize).cast::<u8>(),
                self.len()?,
            )
        };

        Ok(bytes)
    }

    pub fn as_vec(&self) -> JSResult<Vec<u8>> {
        Ok(self.as_bytes()?.to_vec())
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
