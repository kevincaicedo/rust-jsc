use rust_jsc_sys::{
    kJSClassAttributeNoAutomaticPrototype, kJSClassAttributeNone,
    kJSPropertyAttributeDontDelete, kJSPropertyAttributeDontEnum,
    kJSPropertyAttributeNone, kJSPropertyAttributeReadOnly, JSClassAttributes,
    JSClassRef, JSContextGroupRef, JSContextRef, JSGlobalContextRef, JSObjectRef,
    JSPropertyAttributes, JSStringRef, JSType, JSType_kJSTypeBoolean, JSType_kJSTypeNull,
    JSType_kJSTypeNumber, JSType_kJSTypeObject, JSType_kJSTypeString,
    JSType_kJSTypeSymbol, JSType_kJSTypeUndefined, JSTypedArrayType as MJSTypedArrayType,
    JSTypedArrayType_kJSTypedArrayTypeArrayBuffer,
    JSTypedArrayType_kJSTypedArrayTypeBigInt64Array,
    JSTypedArrayType_kJSTypedArrayTypeBigUint64Array,
    JSTypedArrayType_kJSTypedArrayTypeFloat32Array,
    JSTypedArrayType_kJSTypedArrayTypeFloat64Array,
    JSTypedArrayType_kJSTypedArrayTypeInt16Array,
    JSTypedArrayType_kJSTypedArrayTypeInt32Array,
    JSTypedArrayType_kJSTypedArrayTypeInt8Array, JSTypedArrayType_kJSTypedArrayTypeNone,
    JSTypedArrayType_kJSTypedArrayTypeUint16Array,
    JSTypedArrayType_kJSTypedArrayTypeUint32Array,
    JSTypedArrayType_kJSTypedArrayTypeUint8Array,
    JSTypedArrayType_kJSTypedArrayTypeUint8ClampedArray, JSValueRef,
};

use std::{any::TypeId, marker::PhantomData, rc::Rc};

pub mod array;
pub mod class;
pub mod context;
pub mod date;
pub mod error;
pub mod function;
pub mod object;
pub mod promise;
pub mod reg_exp;
pub mod string;
pub mod typed_array;
pub mod value;

pub use rust_jsc_macros::*;

#[doc(hidden)]
pub use rust_jsc_sys as internal;

// re export JSAPIModuleLoader from rust_jsc_sys as JSModuleLoader
pub use rust_jsc_sys::JSAPIModuleLoader as JSModuleLoader;

pub(crate) type NotSendOrSync = PhantomData<Rc<()>>;

pub(crate) fn not_send_or_sync() -> NotSendOrSync {
    PhantomData
}

/// A borrowed JavaScript execution context.
///
/// This handle does not own or release the underlying `JSGlobalContextRef`.
/// Callback macros pass this type into Rust callbacks. Use [`JSGlobalContext`]
/// or [`OwnedJSContext`] when Rust owns the global context lifetime.
/// Context handles are intentionally not `Send` or `Sync`.
#[derive(Clone, Copy)]
pub struct JSContext {
    pub(crate) inner: JSGlobalContextRef,
    pub(crate) _not_send_or_sync: NotSendOrSync,
}

/// An owned JavaScript global context.
///
/// This retains one `JSGlobalContextRef` ownership count and releases it in
/// `Drop`. It dereferences to [`JSContext`] so existing context operations work
/// on `&JSGlobalContext` without runtime overhead.
/// Context handles are intentionally not `Send` or `Sync`.
pub struct JSGlobalContext {
    pub(crate) inner: JSContext,
}

pub type OwnedJSContext = JSGlobalContext;

/// A strictly typed JavaScript context that automatically manages shared data state.
pub struct TypedJSContext<T: 'static> {
    pub(crate) inner: JSGlobalContext,
    _marker: std::marker::PhantomData<T>,
}

pub type PrivateData = *mut ::std::os::raw::c_void;

/// Header-only view of a `TypedData<T>` allocation.
///
/// Because `TypedData<T>` is `#[repr(C)]` with `header` as its first field,
/// casting any `*mut TypedData<T>` to `*const PrivateDataHeader` is valid and
/// lets us inspect the `TypeId` without knowing `T`.
#[repr(C)]
struct PrivateDataHeader {
    type_id: TypeId,
    drop_fn: unsafe fn(*mut std::ffi::c_void),
}

/// Single-allocation, cache-friendly, type-safe wrapper for `*mut c_void`.
///
/// Layout (with `#[repr(C)]`):
/// ```text
/// [ TypeId (8 bytes) | T data (sizeof T, aligned) ]
/// ```
///
/// Benefits:
/// - **One indirection** to reach the data (pointer → contiguous header+data).
/// - **Cache-friendly**: `TypeId` and small `T` share the same cache line.
/// - **No vtable dispatch**: type checking is a direct `TypeId` comparison.
#[repr(C)]
struct TypedData<T> {
    header: PrivateDataHeader,
    data: T,
}

/// Result of installing private or shared data into a JavaScriptCore slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateDataSetStatus {
    /// Data was stored into an empty slot.
    Set,
    /// Existing Rust-owned data was dropped and replaced.
    Replaced,
    /// The slot already had data and the operation did not replace it.
    AlreadySet,
    /// JavaScriptCore rejected the data pointer for this object.
    Unsupported,
}

impl PrivateDataSetStatus {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Set | Self::Replaced)
    }

    pub fn unwrap(self) {
        assert!(self.is_success(), "private data was not stored: {self:?}");
    }
}

/// Result of dropping private or shared data with an expected Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateDataDropStatus {
    /// The slot was empty.
    Empty,
    /// The slot contained data of a different Rust type and was left untouched.
    TypeMismatch,
    /// The data matched the requested type and was dropped.
    Dropped,
}

impl PrivateDataDropStatus {
    pub fn is_dropped(self) -> bool {
        matches!(self, Self::Dropped)
    }
}

/// Result of taking private or shared data with an expected Rust type.
#[derive(Debug, PartialEq, Eq)]
pub enum PrivateDataTakeResult<T> {
    /// The slot was empty.
    Empty,
    /// The slot contained data of a different Rust type and was left untouched.
    TypeMismatch,
    /// The data matched the requested type and ownership was transferred.
    Taken(T),
}

impl<T> PrivateDataTakeResult<T> {
    pub fn is_taken(&self) -> bool {
        matches!(self, Self::Taken(_))
    }

    pub fn is_none(&self) -> bool {
        !self.is_taken()
    }

    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Taken(data) => Some(data),
            Self::Empty | Self::TypeMismatch => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Self::Taken(data) => data,
            Self::Empty => panic!("private data slot is empty"),
            Self::TypeMismatch => panic!("private data type mismatch"),
        }
    }
}

/// Zero-sized namespace for type-safe `*mut c_void` operations.
pub struct PrivateDataWrapper;

impl PrivateDataWrapper {
    /// Create a new wrapper and return a thin `*mut c_void` pointer to it.
    ///
    /// Performs a heap allocation containing `[TypeId | T]`.
    #[inline]
    pub fn into_raw<T: 'static>(data: T) -> *mut std::ffi::c_void {
        let typed = Box::new(TypedData {
            header: PrivateDataHeader {
                type_id: TypeId::of::<T>(),
                drop_fn: Self::drop_typed::<T>,
            },
            data,
        });
        Box::into_raw(typed) as *mut std::ffi::c_void
    }

    unsafe fn drop_typed<T>(ptr: *mut std::ffi::c_void) {
        let _ = unsafe { Box::from_raw(ptr as *mut TypedData<T>) };
    }

    /// Recover a shared reference to the stored data, checking the type at runtime.
    /// Returns `None` if the pointer is null or the type doesn't match.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and must
    /// not have been freed.
    #[inline]
    pub unsafe fn downcast_ref<'a, T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> Option<&'a T> {
        if ptr.is_null() {
            return None;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return None;
        }
        let typed = &*(ptr as *const TypedData<T>);
        Some(&typed.data)
    }

    /// Recover a mutable reference to the stored data, checking the type at runtime.
    /// Returns `None` if the pointer is null or the type doesn't match.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw`, must
    /// not have been freed, and the caller must ensure exclusive access.
    #[inline]
    pub unsafe fn downcast_mut<'a, T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> Option<&'a mut T> {
        if ptr.is_null() {
            return None;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return None;
        }
        let typed = &mut *(ptr as *mut TypedData<T>);
        Some(&mut typed.data)
    }

    /// Take ownership of the stored data, consuming the allocation.
    /// Returns `None` if the pointer is null or the type doesn't match.
    ///
    /// On type mismatch the allocation is **not** freed — the data remains
    /// valid and can be retrieved later with the correct type.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and must
    /// not have been previously freed. On success the pointer becomes invalid.
    #[inline]
    pub unsafe fn take<T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> PrivateDataTakeResult<T> {
        if ptr.is_null() {
            return PrivateDataTakeResult::Empty;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return PrivateDataTakeResult::TypeMismatch;
        }
        let typed = Box::from_raw(ptr as *mut TypedData<T>);
        PrivateDataTakeResult::Taken(typed.data)
    }

    /// Drop the allocation, freeing both the header and the contained `T`.
    ///
    /// The caller must supply the same `T` that was passed to `into_raw`.
    /// If `T` doesn't match, the call is a no-op (the data is not freed).
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and must
    /// not have been previously freed.
    #[allow(dead_code)]
    pub unsafe fn drop_raw<T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> PrivateDataDropStatus {
        if ptr.is_null() {
            return PrivateDataDropStatus::Empty;
        }

        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return PrivateDataDropStatus::TypeMismatch;
        }

        unsafe { (header.drop_fn)(ptr) };
        PrivateDataDropStatus::Dropped
    }

    /// Drop a Rust-owned private-data allocation without statically knowing `T`.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw`, must
    /// not have been previously freed, and no references into the allocation may
    /// be alive.
    pub unsafe fn drop_erased(ptr: *mut std::ffi::c_void) -> PrivateDataDropStatus {
        if ptr.is_null() {
            return PrivateDataDropStatus::Empty;
        }

        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        unsafe { (header.drop_fn)(ptr) };
        PrivateDataDropStatus::Dropped
    }
}

/// A borrowed JavaScript execution context group.
///
/// This handle does not own or release the underlying `JSContextGroupRef`.
/// Use [`OwnedJSContextGroup`] when Rust owns a retained group.
/// Context group handles are intentionally not `Send` or `Sync`.
#[derive(Clone, Copy)]
pub struct JSContextGroup {
    context_group: JSContextGroupRef,
    pub(crate) _not_send_or_sync: NotSendOrSync,
}

/// An owned JavaScript execution context group.
///
/// JavaScriptCore contexts in the same group can share JavaScript values. A
/// group is tied to the run loop of the thread that created it, and using values
/// from the same group across threads requires explicit synchronization.
/// Context group handles are intentionally not `Send` or `Sync`.
pub struct OwnedJSContextGroup {
    pub(crate) inner: JSContextGroup,
}

/// A JavaScript class.
pub struct JSClass {
    // pub(crate) ctx: JSContextRef,
    pub(crate) inner: JSClassRef,
    pub(crate) name: String,
    pub(crate) type_id: TypeId,
}

/// A JavaScript object.
#[derive(Clone)]
pub struct JSObject {
    inner: JSObjectRef,
    value: JSValue,
}

/// A JavaScript function object.
#[derive(Clone)]
pub struct JSFunction {
    pub(crate) object: JSObject,
}

/// A JavaScript date object.
pub struct JSDate {
    pub(crate) object: JSObject,
}

/// A JavaScript regular expression object.
pub struct JSRegExp {
    pub(crate) object: JSObject,
}

/// A JavaScript typed array.
#[derive(Debug, Clone)]
pub struct JSTypedArray {
    pub(crate) object: JSObject,
}

/// A JavaScript array buffer.
#[derive(Debug, Clone)]
pub struct JSArrayBuffer {
    pub(crate) object: JSObject,
}

/// A JavaScript array.
pub struct JSArray {
    pub(crate) object: JSObject,
}

/// A JavaScript promise.
pub struct JSPromise {
    this: JSObject,
    resolver: JSPromiseResolvingFunctions,
}

/// A JavaScript promise resolving functions.
#[derive(Debug, Clone)]
pub struct JSPromiseResolvingFunctions {
    resolve: JSObject,
    reject: JSObject,
}

/// A JavaScript value.
#[derive(Debug, Clone)]
pub struct JSValue {
    pub(crate) inner: JSValueRef,
    pub(crate) ctx: JSContextRef,
}

/// A JavaScript class attribute.
pub enum JSClassAttribute {
    /// Specifies that a class has no special attributes.
    None = kJSClassAttributeNone as isize,
    /// Specifies that a class should not automatically generate a shared prototype for its instance objects.
    /// Use it in combination with set_prototype to manage prototypes manually.
    NoAutomaticPrototype = kJSClassAttributeNoAutomaticPrototype as isize,
}

impl Default for JSClassAttribute {
    fn default() -> Self {
        JSClassAttribute::None
    }
}

impl Into<JSClassAttributes> for JSClassAttribute {
    fn into(self) -> JSClassAttributes {
        self as JSClassAttributes
    }
}

/// A JavaScript value type.
#[derive(Debug, PartialEq)]
pub enum JSValueType {
    Undefined = JSType_kJSTypeUndefined as isize,
    Null = JSType_kJSTypeNull as isize,
    Boolean = JSType_kJSTypeBoolean as isize,
    Number = JSType_kJSTypeNumber as isize,
    String = JSType_kJSTypeString as isize,
    Object = JSType_kJSTypeObject as isize,
    Symbol = JSType_kJSTypeSymbol as isize,
}

impl JSValueType {
    pub(crate) fn from_js_type(value: JSType) -> JSValueType {
        match value {
            x if x == JSType_kJSTypeUndefined => JSValueType::Undefined,
            x if x == JSType_kJSTypeNull => JSValueType::Null,
            x if x == JSType_kJSTypeBoolean => JSValueType::Boolean,
            x if x == JSType_kJSTypeNumber => JSValueType::Number,
            x if x == JSType_kJSTypeString => JSValueType::String,
            x if x == JSType_kJSTypeObject => JSValueType::Object,
            x if x == JSType_kJSTypeSymbol => JSValueType::Symbol,
            x => unreachable!("Unknown JSValue type: {}", x),
        }
    }
}

/// A JavaScript typed array type.
#[derive(Debug, PartialEq)]
pub enum JSTypedArrayType {
    Int8Array = JSTypedArrayType_kJSTypedArrayTypeInt8Array as isize,
    Int16Array = JSTypedArrayType_kJSTypedArrayTypeInt16Array as isize,
    Int32Array = JSTypedArrayType_kJSTypedArrayTypeInt32Array as isize,
    Uint8Array = JSTypedArrayType_kJSTypedArrayTypeUint8Array as isize,
    Uint8ClampedArray = JSTypedArrayType_kJSTypedArrayTypeUint8ClampedArray as isize,
    Uint16Array = JSTypedArrayType_kJSTypedArrayTypeUint16Array as isize,
    Uint32Array = JSTypedArrayType_kJSTypedArrayTypeUint32Array as isize,
    Float32Array = JSTypedArrayType_kJSTypedArrayTypeFloat32Array as isize,
    Float64Array = JSTypedArrayType_kJSTypedArrayTypeFloat64Array as isize,
    ArrayBuffer = JSTypedArrayType_kJSTypedArrayTypeArrayBuffer as isize,
    None = JSTypedArrayType_kJSTypedArrayTypeNone as isize,
    BigInt64Array = JSTypedArrayType_kJSTypedArrayTypeBigInt64Array as isize,
    BigUint64Array = JSTypedArrayType_kJSTypedArrayTypeBigUint64Array as isize,
}

impl Default for JSTypedArrayType {
    fn default() -> Self {
        JSTypedArrayType::None
    }
}

impl Into<MJSTypedArrayType> for JSTypedArrayType {
    fn into(self) -> MJSTypedArrayType {
        self as MJSTypedArrayType
    }
}

impl JSTypedArrayType {
    #[allow(dead_code)]
    pub(crate) fn from_type(value: std::os::raw::c_uint) -> JSTypedArrayType {
        match value {
            x if x == JSTypedArrayType_kJSTypedArrayTypeInt8Array => {
                JSTypedArrayType::Int8Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeInt16Array => {
                JSTypedArrayType::Int16Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeInt32Array => {
                JSTypedArrayType::Int32Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeUint8Array => {
                JSTypedArrayType::Uint8Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeUint8ClampedArray => {
                JSTypedArrayType::Uint8ClampedArray
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeUint16Array => {
                JSTypedArrayType::Uint16Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeUint32Array => {
                JSTypedArrayType::Uint32Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeFloat32Array => {
                JSTypedArrayType::Float32Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeFloat64Array => {
                JSTypedArrayType::Float64Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeArrayBuffer => {
                JSTypedArrayType::ArrayBuffer
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeNone => JSTypedArrayType::None,
            x if x == JSTypedArrayType_kJSTypedArrayTypeBigInt64Array => {
                JSTypedArrayType::BigInt64Array
            }
            x if x == JSTypedArrayType_kJSTypedArrayTypeBigUint64Array => {
                JSTypedArrayType::BigUint64Array
            }
            x => unreachable!("Unknown JSTypedArrayType type: {}", x),
        }
    }
}

/// A JavaScript error.
#[derive(Debug)]
pub struct JSError {
    object: JSObject,
}

/// A JavaScript string.
/// This struct is used to retain a reference to a JavaScript string.
/// It will release the string when it goes out of scope.
pub struct JSString {
    pub(crate) inner: JSStringRef,
}

/// A JavaScript string reference.
/// This struct is used to retain a reference to a JavaScript string.
/// It won't release the string when it goes out of scope.
/// To release the string, use the `release` method.
pub struct JSStringProctected(JSStringRef);

pub type JSResult<T> = Result<T, JSError>;

// A struct to represent a JavaScript property descriptor
#[derive(Debug, Clone, Copy)]
pub struct PropertyDescriptor {
    attributes: JSPropertyAttributes,
}

impl PropertyDescriptor {
    // Constructor to create a new PropertyDescriptor with specified attributes
    pub fn new(attributes: JSPropertyAttributes) -> Self {
        Self { attributes }
    }

    // Check if the property is writable
    pub fn is_writable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeReadOnly) == 0
    }

    /// Check if the property is enumerable
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperty#enumerable
    pub fn is_enumerable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeDontEnum) == 0
    }

    // Check if the property is configurable
    pub fn is_configurable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeDontDelete) == 0
    }
}

impl Default for PropertyDescriptor {
    fn default() -> Self {
        Self {
            attributes: kJSPropertyAttributeNone,
        }
    }
}

// A builder for constructing a set of JavaScript property attributes
pub struct PropertyDescriptorBuilder {
    attributes: JSPropertyAttributes,
}

impl PropertyDescriptorBuilder {
    // Constructor to create a new builder instance
    pub fn new() -> Self {
        Self {
            attributes: kJSPropertyAttributeNone,
        }
    }

    pub fn writable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeReadOnly, value)
    }

    pub fn enumerable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeDontEnum, value)
    }

    pub fn configurable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeDontDelete, value)
    }

    // disable specific attributes could be implemented
    fn set_attribute(mut self, attribute: JSPropertyAttributes, value: bool) -> Self {
        if value {
            self.attributes &= !attribute;
        } else {
            self.attributes |= attribute;
        }
        self
    }

    // Build and retrieve the final attributes
    pub fn build(self) -> PropertyDescriptor {
        PropertyDescriptor {
            attributes: self.attributes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_descriptor_builder() {
        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder
            .writable(true)
            .enumerable(true)
            .configurable(true)
            .build();
        assert_eq!(descriptor.is_writable(), true);
        assert_eq!(descriptor.is_enumerable(), true);
        assert_eq!(descriptor.is_configurable(), true);

        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build();
        assert_eq!(descriptor.is_writable(), false);
        assert_eq!(descriptor.is_enumerable(), false);
        assert_eq!(descriptor.is_configurable(), false);

        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder
            .writable(true)
            .enumerable(false)
            .configurable(true)
            .build();
        assert_eq!(descriptor.is_writable(), true);
        assert_eq!(descriptor.is_enumerable(), false);
        assert_eq!(descriptor.is_configurable(), true);

        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder.build();
        assert_eq!(descriptor.is_writable(), true);
        assert_eq!(descriptor.is_enumerable(), true);
        assert_eq!(descriptor.is_configurable(), true);
    }
}
