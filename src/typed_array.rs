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

use std::ffi::c_void;

use crate::{
    JSArrayBuffer, JSContext, JSError, JSObject, JSResult, JSTypedArray,
    JSTypedArrayType, JSValue,
};

mod sealed {
    pub trait Sealed {}
}

/// Rust element types that can be safely copied to or from a JavaScript typed
/// array backing store.
///
/// This trait is sealed so typed-array views stay limited to primitive numeric
/// layouts with no destructors or invalid drop states.
pub trait JSTypedArrayElement: sealed::Sealed + Copy + Send + 'static {
    fn matches_array_type(array_type: JSTypedArrayType) -> bool;
}

macro_rules! impl_typed_array_element {
    ($rust_type:ty, $($array_type:pat_param)|+ $(,)?) => {
        impl sealed::Sealed for $rust_type {}

        impl JSTypedArrayElement for $rust_type {
            fn matches_array_type(array_type: JSTypedArrayType) -> bool {
                matches!(array_type, $($array_type)|+)
            }
        }
    };
}

impl_typed_array_element!(i8, JSTypedArrayType::Int8Array);
impl_typed_array_element!(i16, JSTypedArrayType::Int16Array);
impl_typed_array_element!(i32, JSTypedArrayType::Int32Array);
impl_typed_array_element!(
    u8,
    JSTypedArrayType::Uint8Array | JSTypedArrayType::Uint8ClampedArray
);
impl_typed_array_element!(u16, JSTypedArrayType::Uint16Array);
impl_typed_array_element!(u32, JSTypedArrayType::Uint32Array);
impl_typed_array_element!(f32, JSTypedArrayType::Float32Array);
impl_typed_array_element!(f64, JSTypedArrayType::Float64Array);
impl_typed_array_element!(i64, JSTypedArrayType::BigInt64Array);
impl_typed_array_element!(u64, JSTypedArrayType::BigUint64Array);

struct ExternalBytes<T: JSTypedArrayElement> {
    bytes: Vec<T>,
}

unsafe extern "C" fn drop_external_bytes<T: JSTypedArrayElement>(
    _bytes: *mut c_void,
    deallocator_context: *mut c_void,
) {
    if deallocator_context.is_null() {
        return;
    }

    // SAFETY: `deallocator_context` is created with `Box::into_raw` from an
    // `ExternalBytes<T>` in this module and is transferred exactly once to JSC.
    unsafe {
        drop(Box::from_raw(
            deallocator_context.cast::<ExternalBytes<T>>(),
        ));
    }
}

fn element_byte_len<T: JSTypedArrayElement>(
    ctx: &JSContext,
    element_len: usize,
) -> JSResult<usize> {
    std::mem::size_of::<T>()
        .checked_mul(element_len)
        .ok_or_else(|| JSError::from_message(ctx, "typed array byte length overflow"))
}

fn validate_typed_array_type<T: JSTypedArrayElement>(
    ctx: &JSContext,
    array_type: JSTypedArrayType,
) -> JSResult<()> {
    if T::matches_array_type(array_type) {
        Ok(())
    } else {
        Err(JSError::from_message(
            ctx,
            "Rust element type does not match JavaScript typed array type",
        ))
    }
}

fn validate_typed_array_storage<T: JSTypedArrayElement>(
    ctx: &JSContext,
    array_type: JSTypedArrayType,
    byte_len: usize,
    ptr: *const T,
) -> JSResult<()> {
    validate_typed_array_type::<T>(ctx, array_type)?;

    let element_size = array_type.element_size().ok_or_else(|| {
        JSError::from_message(ctx, "array_type must be a JavaScript typed array")
    })?;
    if byte_len % element_size != 0 {
        return Err(JSError::from_message(
            ctx,
            "typed array byte length is not aligned to the array element size",
        ));
    }

    if byte_len != 0 && (ptr as usize) % std::mem::align_of::<T>() != 0 {
        return Err(JSError::from_message(
            ctx,
            "typed array bytes pointer is not aligned for the Rust element type",
        ));
    }

    Ok(())
}

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

        // SAFETY: `ctx.inner` is a live context. JavaScriptCore initializes
        // `exception` if typed-array construction throws.
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

        if result.is_null() {
            return Err(JSError::from_message(ctx, "failed to create typed array"));
        }

        let object = JSObject::from_ref(result, ctx.inner);
        Ok(Self { object })
    }

    /// Creates a JSTypedArray from a given JSValue.
    pub fn from_value(value: &JSValue) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let array_type =
            // SAFETY: `value.ctx` and `value.inner` are a live same-context
            // value pair. JavaScriptCore initializes `exception` on failure.
            unsafe { JSValueGetTypedArrayType(value.ctx, value.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, value.ctx);
            return Err(JSError::from(value));
        }

        let array_type = JSTypedArrayType::from_type(array_type);
        if matches!(
            array_type,
            JSTypedArrayType::None | JSTypedArrayType::ArrayBuffer
        ) {
            // SAFETY: `value.ctx` is the live context associated with `value`;
            // this creates a non-owning view for error construction.
            let ctx = unsafe { JSContext::borrowed(value.ctx) };
            return Err(JSError::from_message(
                &ctx,
                "value is not a JavaScript typed array",
            ));
        }

        let object = value.as_object()?;
        Ok(Self { object })
    }

    /// Creates a JavaScript Typed Array object by copying a Rust slice into
    /// JavaScriptCore-owned storage.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the typed array in.
    /// - `bytes`: The data to copy into the typed array.
    /// - `array_type`: The type of the typed array.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while creating the typed array.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// A JSTypedArray that is a Typed Array with a copy of the given data.
    pub fn with_bytes<T: JSTypedArrayElement>(
        ctx: &JSContext,
        bytes: &[T],
        array_type: JSTypedArrayType,
    ) -> JSResult<Self> {
        Self::with_owned_bytes(ctx, bytes.to_vec(), array_type)
    }

    /// Creates a JavaScript Typed Array object by transferring an owned Rust
    /// vector to JavaScriptCore without copying the vector buffer.
    ///
    /// JavaScriptCore calls a Rust deallocator when the backing store is no
    /// longer needed, so the source vector must not be used after this call.
    ///
    /// # Errors
    /// If `array_type` does not match `T`, the byte length is invalid for the
    /// JavaScript element type, the buffer is misaligned, or JavaScriptCore
    /// throws while creating the typed array.
    pub fn with_owned_bytes<T: JSTypedArrayElement>(
        ctx: &JSContext,
        bytes: Vec<T>,
        array_type: JSTypedArrayType,
    ) -> JSResult<Self> {
        let byte_len = element_byte_len::<T>(ctx, bytes.len())?;
        validate_typed_array_storage(ctx, array_type, byte_len, bytes.as_ptr())?;

        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut external = Box::new(ExternalBytes { bytes });
        let ptr = external.bytes.as_mut_ptr().cast::<c_void>();
        let deallocator_context = Box::into_raw(external).cast::<c_void>();

        // SAFETY: `ptr` points to the buffer owned by `deallocator_context`.
        // JSC receives the matching Rust deallocator and will release the boxed
        // vector when the typed-array backing store is destroyed.
        let result = unsafe {
            JSObjectMakeTypedArrayWithBytesNoCopy(
                ctx.inner,
                array_type as _,
                ptr,
                byte_len,
                Some(drop_external_bytes::<T>),
                deallocator_context,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::from_message(
                ctx,
                "failed to create typed array from owned bytes",
            ));
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }

    /// Creates a JavaScript Typed Array object from caller-owned bytes without
    /// copying.
    ///
    /// Prefer [`JSTypedArray::with_bytes`] or [`JSTypedArray::with_owned_bytes`]
    /// unless the caller owns a backing store with a lifetime that is already
    /// tied to the JavaScript object graph.
    ///
    /// # Safety
    /// The caller must keep `bytes` allocated, pinned at the same address, and
    /// exclusively available to JavaScriptCore until every JS typed-array or
    /// ArrayBuffer view that can reach it is gone. Rust must not read or write
    /// the slice while JavaScriptCore may access it, and the storage must not be
    /// freed by Rust before JSC stops using it.
    ///
    /// # Errors
    /// If `array_type` does not match `T`, the byte length is invalid for the
    /// JavaScript element type, the buffer is misaligned, or JavaScriptCore
    /// throws while creating the typed array.
    pub unsafe fn with_bytes_no_copy<T: JSTypedArrayElement>(
        ctx: &JSContext,
        bytes: &mut [T],
        array_type: JSTypedArrayType,
    ) -> JSResult<Self> {
        let byte_len = element_byte_len::<T>(ctx, bytes.len())?;
        validate_typed_array_storage(ctx, array_type, byte_len, bytes.as_ptr())?;

        let mut exception: JSValueRef = std::ptr::null_mut();

        // SAFETY: the caller promises the borrowed backing store remains valid
        // for JavaScriptCore until all JS views that can reach it are gone.
        let result = unsafe {
            JSObjectMakeTypedArrayWithBytesNoCopy(
                ctx.inner,
                array_type as _,
                bytes.as_mut_ptr() as _,
                byte_len,
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
            return Err(JSError::from_message(
                ctx,
                "failed to create typed array from bytes",
            ));
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
        // SAFETY: `self.object` holds a live object/context pair.
        // JavaScriptCore initializes `exception` on failure.
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
        // SAFETY: `self.object` holds a live typed-array object/context pair.
        // JavaScriptCore initializes `exception` on failure.
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

    /// Returns true when the typed array has no elements.
    ///
    /// # Errors
    /// If JavaScriptCore throws while reading the typed-array length.
    pub fn is_empty(&self) -> JSResult<bool> {
        self.len().map(|len| len == 0)
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
        // SAFETY: `self.object` holds a live typed-array object/context pair.
        // JavaScriptCore initializes `exception` on failure.
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
        // SAFETY: `self.object` holds a live typed-array object/context pair.
        // JavaScriptCore initializes `exception` on failure.
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
        // SAFETY: `self.object` holds a live typed-array object/context pair.
        // JavaScriptCore initializes `exception` on failure and returns a
        // borrowed ArrayBuffer object handle on success.
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

        if result.is_null() {
            // SAFETY: `self.object.ctx` is the live context associated with the
            // typed-array wrapper; this borrowed view does not release it.
            let context = unsafe { JSContext::borrowed(self.object.ctx) };
            return Err(JSError::from_message(
                &context,
                "failed to get typed array buffer",
            ));
        }

        Ok(JSArrayBuffer::from_object(JSObject::from_ref(
            result,
            self.object.ctx,
        )))
    }

    /// Gets a temporary mutable typed view into the Typed Array backing store.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// assert_eq!(typed_array.len().unwrap(), 5);
    /// assert_eq!(typed_array.byte_len().unwrap(), 5);
    /// assert_eq!(typed_array.byte_offset().unwrap(), 0);
    /// // SAFETY: the temporary JSC pointer is used only for this immediate length read.
    /// assert_eq!(unsafe { typed_array.bytes::<u8>().unwrap().len() }, 5);
    /// ```
    ///
    /// # Safety
    /// JavaScriptCore documents this pointer as temporary and not guaranteed to
    /// stay valid across JavaScriptCore API calls. The caller must not call any
    /// JSC API while the returned slice is live, must not create another Rust
    /// alias to the same backing store, and must use the matching Rust element
    /// type for the JavaScript typed array.
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the Typed Array object or `null` if the object is not a Typed Array object.
    pub unsafe fn bytes<T: JSTypedArrayElement>(&self) -> JSResult<&mut [T]> {
        // SAFETY: `self.object.ctx` is the live context associated with this
        // wrapper; this borrowed view does not release it.
        let context = unsafe { JSContext::borrowed(self.object.ctx) };
        validate_typed_array_type::<T>(&context, self.array_type()?)?;
        let byte_len = self.byte_len()?;
        let byte_offset = self.byte_offset()?;

        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.object` is a JavaScriptCore typed-array object wrapper.
        // The returned pointer is checked below and exposed only through this
        // unsafe API because JSC makes it temporary.
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

        let element_size = std::mem::size_of::<T>();
        if byte_len % element_size != 0 {
            return Err(JSError::from_message(
                &context,
                "typed array byte length is not a multiple of the requested Rust element size",
            ));
        }
        if result.is_null() {
            return Err(JSError::from_message(
                &context,
                "typed array bytes pointer is null",
            ));
        }

        // SAFETY: `result` is non-null and `byte_offset` is reported by JSC for
        // this typed array.
        let ptr = unsafe { result.cast::<u8>().add(byte_offset).cast::<T>() };
        if (ptr as usize) % std::mem::align_of::<T>() != 0 {
            return Err(JSError::from_message(
                &context,
                "typed array bytes pointer is not aligned for the requested Rust element type",
            ));
        }
        let bytes =
            // SAFETY: type, byte length, null pointer, and alignment were
            // checked above. Lifetime and aliasing are the caller's
            // responsibility for this unsafe API.
            unsafe { std::slice::from_raw_parts_mut(ptr, byte_len / element_size) };

        Ok(bytes)
    }

    /// Gets a temporary mutable typed view from a raw `JSValue`.
    ///
    /// # Safety
    /// The returned pointer has the same temporary JavaScriptCore lifetime as
    /// [`JSTypedArray::bytes`]. The caller must not call any JSC API while the
    /// returned slice is live, must avoid aliasing the backing store, and must
    /// use the matching Rust element type for the JavaScript typed array.
    pub unsafe fn bytes_from_value<T: JSTypedArrayElement>(
        value: &JSValue,
    ) -> JSResult<&mut [T]> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let array_type_raw =
            // SAFETY: `value.inner` belongs to `value.ctx`; exceptions are checked.
            unsafe { JSValueGetTypedArrayType(value.ctx, value.inner, &mut exception) };
        if !exception.is_null() {
            let value = JSValue::new(exception, value.ctx);
            return Err(JSError::from(value));
        }

        // SAFETY: `value.ctx` is the live context associated with `value`; this
        // borrowed view does not release it.
        let context = unsafe { JSContext::borrowed(value.ctx) };
        validate_typed_array_type::<T>(
            &context,
            JSTypedArrayType::from_type(array_type_raw),
        )?;

        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut offset: usize = 0;
        let mut len: usize = 0;

        // SAFETY: `value.inner` belongs to `value.ctx`; exceptions are checked.
        // The returned pointer is exposed only through this unsafe API because
        // JSC makes it temporary.
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
            return Err(JSError::from_message(
                &context,
                "typed array bytes pointer is null",
            ));
        }

        let element_size = std::mem::size_of::<T>();
        if len % element_size != 0 {
            return Err(JSError::from_message(
                &context,
                "typed array byte length is not a multiple of the requested Rust element size",
            ));
        }
        // SAFETY: `result` is non-null and `offset` is reported by JSC for this
        // typed array value.
        let ptr = unsafe { result.cast::<u8>().add(offset).cast::<T>() };
        if (ptr as usize) % std::mem::align_of::<T>() != 0 {
            return Err(JSError::from_message(
                &context,
                "typed array bytes pointer is not aligned for the requested Rust element type",
            ));
        }

        // SAFETY: type, byte length, null pointer, and alignment were checked
        // above. Lifetime and aliasing are the caller's responsibility for this
        // unsafe API.
        let bytes = unsafe { std::slice::from_raw_parts_mut(ptr, len / element_size) };

        Ok(bytes)
    }

    /// Gets the bytes of the Typed Array as a Vec.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSTypedArray, JSTypedArrayType};
    ///
    /// let ctx = JSContext::new();
    /// let bytes = vec![6, 5, 5, 6, 9];
    /// let typed_array = JSTypedArray::with_bytes::<u8>(&ctx, bytes.as_slice(), JSTypedArrayType::Uint8Array).unwrap();
    /// assert_eq!(typed_array.as_vec::<u8>().unwrap(), &[6, 5, 5, 6, 9]);
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the Typed Array object as a Vec or `null` if the object is not a Typed Array object.
    pub fn as_vec<T: JSTypedArrayElement>(&self) -> JSResult<Vec<T>> {
        // SAFETY: the temporary JSC pointer is copied into an owned Vec before
        // this method makes any further JavaScriptCore calls or returns.
        Ok(unsafe { self.bytes::<T>()? }.to_vec())
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
        // SAFETY: `ctx.inner` and `array_buffer.object.inner` are live handles.
        // JavaScriptCore initializes `exception` on failure and creates a new
        // typed-array view on success.
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

        if result.is_null() {
            return Err(JSError::from_message(
                ctx,
                "failed to create typed array with buffer",
            ));
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
        let buffer_len = array_buffer.len()?;
        if byte_offset > buffer_len {
            return Err(JSError::from_message(
                ctx,
                "typed array byte offset exceeds array buffer length",
            ));
        }
        let element_size = array_type.element_size().ok_or_else(|| {
            JSError::from_message(ctx, "array_type must be a JavaScript typed array")
        })?;
        let byte_length = buffer_len - byte_offset;
        if byte_length % element_size != 0 {
            return Err(JSError::from_message(
                ctx,
                "typed array byte length is not aligned to the array element size",
            ));
        }
        let element_length = byte_length / element_size;
        // SAFETY: `ctx.inner` and `array_buffer.object.inner` are live handles.
        // Offset/length were validated against the buffer and element size.
        // JavaScriptCore initializes `exception` on failure.
        let result = unsafe {
            JSObjectMakeTypedArrayWithArrayBufferAndOffset(
                ctx.inner,
                array_type as _,
                array_buffer.object.inner,
                byte_offset as _,
                element_length as _,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::from_message(
                ctx,
                "failed to create typed array with buffer and offset",
            ));
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
        // SAFETY: `self.object` holds a live ArrayBuffer object/context pair.
        // JavaScriptCore initializes `exception` on failure.
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

    /// Returns true when the ArrayBuffer has zero bytes.
    ///
    /// # Errors
    /// If JavaScriptCore throws while reading the ArrayBuffer byte length.
    pub fn is_empty(&self) -> JSResult<bool> {
        self.len().map(|len| len == 0)
    }

    /// Gets a temporary mutable byte view into the ArrayBuffer backing store.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
    /// let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());
    /// // SAFETY: the temporary JSC pointer is used only for this immediate length read.
    /// assert_eq!(unsafe { array_buffer.bytes().unwrap().len() }, 10);
    /// ```
    ///
    /// # Safety
    /// JavaScriptCore documents this pointer as temporary and not guaranteed to
    /// stay valid across JavaScriptCore API calls. The caller must not call any
    /// JSC API while the returned slice is live and must not create another Rust
    /// alias to the same backing store.
    ///
    /// # Errors
    /// If an exception is thrown while getting the bytes.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The bytes of the ArrayBuffer object or `null` if the object is not an ArrayBuffer object.
    pub unsafe fn bytes(&self) -> JSResult<&mut [u8]> {
        let len = self.len()?;

        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.object` is an ArrayBuffer wrapper. The returned pointer
        // is checked below and exposed only through this unsafe API because JSC
        // makes it temporary.
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

        if result.is_null() {
            // SAFETY: `self.object.ctx` is the live context associated with the
            // ArrayBuffer wrapper; this borrowed view does not release it.
            let context = unsafe { JSContext::borrowed(self.object.ctx) };
            return Err(JSError::from_message(
                &context,
                "array buffer bytes pointer is null",
            ));
        }

        // SAFETY: `result` is non-null and `len` was reported by JSC for this
        // ArrayBuffer. Lifetime and aliasing are the caller's responsibility for
        // this unsafe API.
        let bytes = unsafe { std::slice::from_raw_parts_mut(result as *mut u8, len) };

        Ok(bytes)
    }

    /// Checks if the ArrayBuffer is detached.
    /// Detached ArrayBuffers are ArrayBuffers that have been detached from their backing store.
    /// This can happen when the backing store is transferred to another object.
    pub fn is_detached(&self) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.object` holds a live ArrayBuffer object/context pair.
        // JavaScriptCore initializes `exception` on failure.
        let result = unsafe {
            JSObjectIsDetachedBuffer(self.object.ctx, self.object.inner, &mut exception)
        };
        if !exception.is_null() {
            let value = JSValue::new(exception, self.object.ctx);
            return Err(JSError::from(value));
        }
        Ok(result)
    }

    /// Creates a new `JSArrayBuffer` by copying bytes into JavaScriptCore-owned
    /// storage.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSContext, JSArrayBuffer};
    ///
    /// let ctx = JSContext::new();
    /// let bytes = vec![6; 10];
    /// let array_buffer = JSArrayBuffer::from_bytes(&ctx, bytes.as_slice()).unwrap();
    /// assert_eq!(array_buffer.as_vec().unwrap(), vec![6; 10]);
    /// ```
    ///
    /// # Errors
    /// If JavaScriptCore throws while creating the ArrayBuffer.
    pub fn from_bytes(ctx: &JSContext, bytes: &[u8]) -> JSResult<Self> {
        Self::from_vec(ctx, bytes.to_vec())
    }

    /// Creates a new `JSArrayBuffer` by transferring an owned byte vector to
    /// JavaScriptCore without copying the vector buffer.
    ///
    /// JavaScriptCore calls a Rust deallocator when the backing store is no
    /// longer needed, so the source vector must not be used after this call.
    ///
    /// # Errors
    /// If JavaScriptCore throws while creating the ArrayBuffer.
    pub fn from_vec(ctx: &JSContext, bytes: Vec<u8>) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let byte_len = bytes.len();
        let mut external = Box::new(ExternalBytes { bytes });
        let ptr = external.bytes.as_mut_ptr().cast::<c_void>();
        let deallocator_context = Box::into_raw(external).cast::<c_void>();

        // SAFETY: `ptr` points to the buffer owned by `deallocator_context`.
        // JSC receives the matching Rust deallocator and will release the boxed
        // vector when the ArrayBuffer backing store is destroyed.
        let result = unsafe {
            JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.inner,
                ptr,
                byte_len,
                Some(drop_external_bytes::<u8>),
                deallocator_context,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            return Err(JSError::from_message(ctx, "failed to create array buffer"));
        }

        Ok(Self {
            object: JSObject::from_ref(result, ctx.inner),
        })
    }

    /// Creates a new `JSArrayBuffer` by copying bytes into JavaScriptCore-owned
    /// storage.
    ///
    /// Prefer [`JSArrayBuffer::from_bytes`] in new code; this method remains as
    /// the historical constructor name.
    ///
    /// # Errors
    /// If JavaScriptCore throws while creating the ArrayBuffer.
    pub fn new(ctx: &JSContext, bytes: &[u8]) -> JSResult<Self> {
        Self::from_bytes(ctx, bytes)
    }

    /// Creates a JavaScript ArrayBuffer from caller-owned bytes without
    /// copying.
    ///
    /// Prefer [`JSArrayBuffer::from_bytes`] or [`JSArrayBuffer::from_vec`]
    /// unless the caller owns a backing store with a lifetime that is already
    /// tied to the JavaScript object graph.
    ///
    /// # Safety
    /// The caller must keep `bytes` allocated, pinned at the same address, and
    /// exclusively available to JavaScriptCore until every JS ArrayBuffer or
    /// typed-array view that can reach it is gone. Rust must not read or write
    /// the slice while JavaScriptCore may access it, and the storage must not be
    /// freed by Rust before JSC stops using it.
    ///
    /// # Errors
    /// If JavaScriptCore throws while creating the ArrayBuffer.
    pub unsafe fn with_bytes_no_copy(
        ctx: &JSContext,
        bytes: &mut [u8],
    ) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();

        // SAFETY: the caller promises the borrowed backing store remains valid
        // for JavaScriptCore until all JS views that can reach it are gone.
        let result = unsafe {
            JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.inner,
                bytes.as_mut_ptr().cast::<c_void>(),
                bytes.len(),
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
            return Err(JSError::from_message(ctx, "failed to create array buffer"));
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
        // SAFETY: the temporary JSC pointer is copied into an owned Vec before
        // this method makes any further JavaScriptCore calls or returns.
        Ok(unsafe { self.bytes()? }.to_vec())
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
            bytes.as_slice(),
            JSTypedArrayType::Uint8Array,
        )
        .unwrap();
        bytes[1] = 9;
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

        // SAFETY: the test copies from the temporary JSC bytes before making
        // any further JavaScriptCore calls.
        let result = unsafe { typed_array.bytes::<u8>().unwrap() };
        assert_eq!(result.len(), 3);
        assert_eq!(result, &[5, 5, 6,]);
    }

    #[test]
    fn test_typed_array_with_owned_bytes_transfers_storage() {
        let ctx = JSContext::new();
        let typed_array = JSTypedArray::with_owned_bytes::<u16>(
            &ctx,
            vec![1u16, 2, 3, 4],
            JSTypedArrayType::Uint16Array,
        )
        .unwrap();

        assert_eq!(typed_array.len().unwrap(), 4);
        assert_eq!(typed_array.byte_len().unwrap(), 8);
        assert_eq!(typed_array.as_vec::<u16>().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_typed_array_rejects_mismatched_rust_element_type() {
        let ctx = JSContext::new();
        let error = JSTypedArray::with_bytes::<u8>(
            &ctx,
            &[1, 2, 3, 4],
            JSTypedArrayType::Uint16Array,
        )
        .unwrap_err();

        assert_eq!(
            error.message().unwrap().to_string(),
            "Rust element type does not match JavaScript typed array type"
        );
    }

    #[test]
    fn test_typed_array_with_bytes_offset() {
        let ctx = JSContext::new();
        let mut bytes = vec![6u16; 5];
        let typed_array = JSTypedArray::with_bytes::<u16>(
            &ctx,
            bytes.as_slice(),
            JSTypedArrayType::Uint16Array,
        )
        .unwrap();
        bytes[0] = 9;
        assert_eq!(
            typed_array.array_type().unwrap(),
            JSTypedArrayType::Uint16Array
        );
        assert_eq!(typed_array.len().unwrap(), 5);
        assert_eq!(typed_array.byte_len().unwrap(), 10);
        assert_eq!(typed_array.byte_offset().unwrap(), 0);
        // SAFETY: the test reads only the temporary slice length before making
        // any further JavaScriptCore calls.
        assert_eq!(unsafe { typed_array.bytes::<u16>().unwrap().len() }, 5);
        assert_eq!(typed_array.as_vec::<u16>().unwrap(), vec![6u16; 5]);

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
    fn test_typed_array_from_value_rejects_non_typed_array() {
        let ctx = JSContext::new();
        let value = ctx.evaluate_script("({ byteLength: 4 })", None).unwrap();

        let error = JSTypedArray::from_value(&value).unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "value is not a JavaScript typed array"
        );
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
        let bytes = vec![6; 10];
        let array_buffer = JSArrayBuffer::new(&ctx, bytes.as_slice()).unwrap();
        assert_eq!(array_buffer.len().unwrap(), 10);
        assert_eq!(array_buffer.as_vec().unwrap(), vec![6; 10]);
    }

    #[test]
    fn test_array_buffer_with_bytes() {
        let ctx = JSContext::new();
        let array_buffer = ctx.evaluate_script("new ArrayBuffer(10)", None).unwrap();
        let array_buffer = JSArrayBuffer::from_object(array_buffer.as_object().unwrap());

        assert_eq!(array_buffer.len().unwrap(), 10);
    }

    #[test]
    fn test_array_buffer_from_bytes_copies_source() {
        let ctx = JSContext::new();
        let mut bytes = vec![1u8, 2, 3, 4];
        let array_buffer = JSArrayBuffer::from_bytes(&ctx, bytes.as_slice()).unwrap();
        bytes[0] = 9;

        assert_eq!(array_buffer.as_vec().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_array_buffer_from_vec_transfers_storage() {
        let ctx = JSContext::new();
        let array_buffer = JSArrayBuffer::from_vec(&ctx, vec![4u8, 3, 2, 1]).unwrap();

        assert_eq!(array_buffer.len().unwrap(), 4);
        assert_eq!(array_buffer.as_vec().unwrap(), vec![4, 3, 2, 1]);
    }

    #[test]
    fn test_array_buffer_is_detached() {
        let ctx = JSContext::new();
        let bytes = vec![6; 10];
        let array_buffer = JSArrayBuffer::new(&ctx, bytes.as_slice()).unwrap();
        assert!(!array_buffer.is_detached().unwrap());

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
        assert!(array_buffer.is_detached().unwrap());
    }

    #[test]
    fn test_array_buffer_bytes_returns_error_for_non_buffer() {
        let ctx = JSContext::new();
        let object = ctx
            .evaluate_script("({})", None)
            .unwrap()
            .as_object()
            .unwrap();
        let array_buffer = JSArrayBuffer::from_object(object);

        // SAFETY: this test immediately observes the error result and does not
        // keep a borrowed backing-store slice alive.
        assert!(unsafe { array_buffer.bytes() }.is_err());
    }

    #[test]
    fn test_typed_array_get_buffer_returns_error_for_non_typed_array() {
        let ctx = JSContext::new();
        let object = ctx
            .evaluate_script("({})", None)
            .unwrap()
            .as_object()
            .unwrap();
        let typed_array = JSTypedArray::from(object);

        assert!(typed_array.get_buffer().is_err());
    }
}
