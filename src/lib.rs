use rust_jsc_sys::{
    kJSClassAttributeNoAutomaticPrototype, kJSClassAttributeNone,
    kJSPropertyAttributeDontDelete, kJSPropertyAttributeDontEnum,
    kJSPropertyAttributeNone, kJSPropertyAttributeReadOnly, JSClassAttributes,
    JSClassRef, JSContextGroupRef, JSContextRef, JSGlobalContextRef,
    JSObjectCallAsFunctionCallback, JSObjectRef, JSPropertyAttributes, JSStringRef,
    JSType, JSType_kJSTypeBoolean, JSType_kJSTypeNull, JSType_kJSTypeNumber,
    JSType_kJSTypeObject, JSType_kJSTypeString, JSType_kJSTypeSymbol,
    JSType_kJSTypeUndefined, JSTypedArrayType as MJSTypedArrayType,
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

use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    collections::HashSet,
    marker::PhantomData,
    ops::Deref,
    rc::Rc,
};

pub mod array;
pub mod class;
pub mod context;
pub mod conversion;
pub mod date;
pub mod error;
pub mod function;
pub mod module_loader;
pub mod object;
pub mod promise;
pub mod reg_exp;
pub mod string;
pub mod typed_array;
pub mod value;

pub use rust_jsc_macros::*;

pub use class::{ClassError, JSClassAccessor, JSClassBuilder};
pub use context::{
    InspectorInboundMessage, InspectorMessageError, InspectorMessageHandler,
    InspectorOutboundMessage, InspectorPauseEvent, InspectorPauseEventHandler,
    InspectorSession, InspectorSessionBuilder, OwnedInspectorMessage,
};
pub use conversion::{FromJSValue, IntoJSResult, IntoJSValue, Rest, TryFromJSValue};

#[doc(hidden)]
pub use rust_jsc_sys as internal;

// re export JSAPIModuleLoader from rust_jsc_sys as JSModuleLoader
pub use rust_jsc_sys::JSAPIModuleLoader as JSModuleLoader;

pub use module_loader::{
    IntoImportMetaResult, IntoModuleResolveResult, IntoModuleSourceResult,
    JSModuleSource, ModuleImportType, ModuleLoadError, ModuleLoader, ModuleLoaderBuilder,
    ModuleSource,
};
pub use typed_array::JSTypedArrayElement;

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
    borrow: Cell<isize>,
}

/// Single-allocation, cache-friendly, type-safe wrapper for `*mut c_void`.
///
/// Layout (with `#[repr(C)]`):
/// ```text
/// [ TypeId | drop fn | borrow flag | T data (sizeof T, aligned) ]
/// ```
///
/// Benefits:
/// - **One indirection** to reach the data (pointer → contiguous header+data).
/// - **Cache-friendly**: `TypeId` and small `T` share the same cache line.
/// - **No vtable dispatch**: type checking is a direct `TypeId` comparison.
/// - **No locks or atomics**: borrow tracking is local to the JS thread.
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
    /// The slot is currently borrowed and the operation did not modify it.
    Borrowed,
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
    /// The slot is currently borrowed and was left untouched.
    Borrowed,
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
    /// The slot is currently borrowed and was left untouched.
    Borrowed,
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
            Self::Empty | Self::TypeMismatch | Self::Borrowed => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Self::Taken(data) => data,
            Self::Empty => panic!("private data slot is empty"),
            Self::TypeMismatch => panic!("private data type mismatch"),
            Self::Borrowed => panic!("private data is currently borrowed"),
        }
    }
}

const PRIVATE_DATA_MUT_BORROW: isize = -1;

thread_local! {
    static PRIVATE_DATA_REGISTRY: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

fn register_private_data(ptr: *mut std::ffi::c_void) {
    PRIVATE_DATA_REGISTRY.with(|registry| {
        registry.borrow_mut().insert(ptr as usize);
    });
}

fn unregister_private_data(ptr: *mut std::ffi::c_void) -> bool {
    PRIVATE_DATA_REGISTRY.with(|registry| registry.borrow_mut().remove(&(ptr as usize)))
}

fn is_registered_private_data(ptr: *mut std::ffi::c_void) -> bool {
    PRIVATE_DATA_REGISTRY.with(|registry| registry.borrow().contains(&(ptr as usize)))
}

impl PrivateDataHeader {
    #[inline]
    fn try_borrow(&self) -> bool {
        let current = self.borrow.get();
        if current == PRIVATE_DATA_MUT_BORROW {
            return false;
        }

        self.borrow.set(current + 1);
        true
    }

    #[inline]
    fn release_borrow(&self) {
        let current = self.borrow.get();
        debug_assert!(current > 0);
        self.borrow.set(current - 1);
    }

    #[inline]
    fn try_borrow_mut(&self) -> bool {
        if self.borrow.get() != 0 {
            return false;
        }

        self.borrow.set(PRIVATE_DATA_MUT_BORROW);
        true
    }

    #[inline]
    fn release_borrow_mut(&self) {
        debug_assert_eq!(self.borrow.get(), PRIVATE_DATA_MUT_BORROW);
        self.borrow.set(0);
    }

    #[inline]
    fn is_borrowed(&self) -> bool {
        self.borrow.get() != 0
    }
}

/// Shared RAII borrow of Rust data stored in a JavaScriptCore private-data slot.
///
/// Dropping this guard releases the runtime borrow. While shared borrows are
/// alive, mutable borrows, safe takes, safe drops, and safe replacements return
/// a borrowed status instead of invalidating the referenced allocation.
pub struct PrivateDataRef<'a, T: 'static> {
    ptr: *mut std::ffi::c_void,
    data: *const T,
    _marker: PhantomData<&'a T>,
    _not_send_or_sync: NotSendOrSync,
}

impl<T: 'static> Deref for PrivateDataRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is created only after type checking and acquiring a
        // shared borrow. Safe take/drop/replace operations refuse borrowed data.
        unsafe { &*self.data }
    }
}

impl<T: 'static> Drop for PrivateDataRef<'_, T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `PrivateDataWrapper::into_raw` and the guard
        // holds one shared borrow that must be released exactly once.
        unsafe { PrivateDataWrapper::release_borrow(self.ptr) };
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for PrivateDataRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: 'static> AsRef<T> for PrivateDataRef<'_, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<'b, T> PartialEq<PrivateDataRef<'b, T>> for PrivateDataRef<'_, T>
where
    T: PartialEq + 'static,
{
    fn eq(&self, other: &PrivateDataRef<'b, T>) -> bool {
        **self == **other
    }
}

impl<T> PartialEq<T> for PrivateDataRef<'_, T>
where
    T: PartialEq + 'static,
{
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
}

impl PartialEq<&str> for PrivateDataRef<'_, String> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl<T: Eq + 'static> Eq for PrivateDataRef<'_, T> {}

/// Exclusive RAII borrow of Rust data stored in a JavaScriptCore private-data slot.
///
/// Dropping this guard releases the runtime mutable borrow.
pub struct PrivateDataMut<'a, T: 'static> {
    ptr: *mut std::ffi::c_void,
    data: *mut T,
    _marker: PhantomData<&'a mut T>,
    _not_send_or_sync: NotSendOrSync,
}

impl<T: 'static> Deref for PrivateDataMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is created only after acquiring the exclusive
        // runtime borrow and stores the checked `T` pointer.
        unsafe { &*self.data }
    }
}

impl<T: 'static> std::ops::DerefMut for PrivateDataMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: this guard owns the only active mutable runtime borrow.
        unsafe { &mut *self.data }
    }
}

impl<T: 'static> Drop for PrivateDataMut<'_, T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `PrivateDataWrapper::into_raw` and the guard
        // holds one mutable borrow that must be released exactly once.
        unsafe { PrivateDataWrapper::release_borrow_mut(self.ptr) };
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for PrivateDataMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: 'static> AsRef<T> for PrivateDataMut<'_, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: 'static> AsMut<T> for PrivateDataMut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self
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
                borrow: Cell::new(0),
            },
            data,
        });
        let ptr = Box::into_raw(typed) as *mut std::ffi::c_void;
        register_private_data(ptr);
        ptr
    }

    unsafe fn drop_typed<T>(ptr: *mut std::ffi::c_void) {
        unregister_private_data(ptr);
        // SAFETY: callers pass a pointer allocated by `into_raw::<T>` that has
        // not already been freed; reconstructing the box drops `T` and header.
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
        if !is_registered_private_data(ptr) {
            return None;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return None;
        }
        let typed = &*(ptr as *const TypedData<T>);
        Some(&typed.data)
    }

    /// Acquire a shared runtime borrow of the stored data.
    ///
    /// Returns `None` if the pointer is null, the type does not match, or the
    /// data is currently mutably borrowed.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and
    /// must not have been freed.
    #[inline]
    pub unsafe fn borrow_ref<'a, T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> Option<PrivateDataRef<'a, T>> {
        if ptr.is_null() {
            return None;
        }
        if !is_registered_private_data(ptr) {
            return None;
        }
        // SAFETY: the caller guarantees `ptr` came from `into_raw` and remains
        // allocated. Reading the header does not move or free the allocation.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        if header.type_id != TypeId::of::<T>() || !header.try_borrow() {
            return None;
        }
        // SAFETY: the header type check above proves the allocation layout is
        // `TypedData<T>`, and the borrow flag now records a shared borrow.
        let typed = unsafe { &*(ptr as *const TypedData<T>) };
        Some(PrivateDataRef {
            ptr,
            data: &typed.data,
            _marker: PhantomData,
            _not_send_or_sync: not_send_or_sync(),
        })
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
        if !is_registered_private_data(ptr) {
            return None;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return None;
        }
        let typed = &mut *(ptr as *mut TypedData<T>);
        Some(&mut typed.data)
    }

    /// Acquire an exclusive runtime borrow of the stored data.
    ///
    /// Returns `None` if the pointer is null, the type does not match, or any
    /// runtime borrow is currently active.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and
    /// must not have been freed.
    #[inline]
    pub unsafe fn borrow_mut<'a, T: 'static>(
        ptr: *mut std::ffi::c_void,
    ) -> Option<PrivateDataMut<'a, T>> {
        if ptr.is_null() {
            return None;
        }
        if !is_registered_private_data(ptr) {
            return None;
        }
        // SAFETY: the caller guarantees `ptr` came from `into_raw` and remains
        // allocated. Reading the header does not move or free the allocation.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        if header.type_id != TypeId::of::<T>() || !header.try_borrow_mut() {
            return None;
        }
        // SAFETY: the header type check above proves the allocation layout is
        // `TypedData<T>`, and the borrow flag now records exclusive access.
        let typed = unsafe { &mut *(ptr as *mut TypedData<T>) };
        Some(PrivateDataMut {
            ptr,
            data: &mut typed.data,
            _marker: PhantomData,
            _not_send_or_sync: not_send_or_sync(),
        })
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
        if !is_registered_private_data(ptr) {
            return PrivateDataTakeResult::TypeMismatch;
        }
        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return PrivateDataTakeResult::TypeMismatch;
        }
        if header.is_borrowed() {
            return PrivateDataTakeResult::Borrowed;
        }
        unregister_private_data(ptr);
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
        if !is_registered_private_data(ptr) {
            return PrivateDataDropStatus::TypeMismatch;
        }

        let header = &*(ptr as *const PrivateDataHeader);
        if header.type_id != TypeId::of::<T>() {
            return PrivateDataDropStatus::TypeMismatch;
        }
        if header.is_borrowed() {
            return PrivateDataDropStatus::Borrowed;
        }

        // SAFETY: the header type check proves `drop_fn` matches the allocation
        // type, and the borrow check above proves no live guarded references.
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
        if !is_registered_private_data(ptr) {
            return PrivateDataDropStatus::TypeMismatch;
        }

        // SAFETY: the caller guarantees `ptr` came from `into_raw` and remains
        // allocated. Reading the header does not move or free the allocation.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        if header.is_borrowed() {
            return PrivateDataDropStatus::Borrowed;
        }
        // SAFETY: `drop_fn` is stored in the allocation header by `into_raw`
        // and matches the concrete type. No runtime borrow is active.
        unsafe { (header.drop_fn)(ptr) };
        PrivateDataDropStatus::Dropped
    }

    /// Returns true when any shared or mutable borrow is active.
    ///
    /// # Safety
    /// The pointer must have been created by `PrivateDataWrapper::into_raw` and
    /// must not have been freed.
    #[inline]
    pub unsafe fn is_borrowed(ptr: *mut std::ffi::c_void) -> bool {
        if ptr.is_null() {
            return false;
        }
        if !is_registered_private_data(ptr) {
            return false;
        }

        // SAFETY: the caller guarantees `ptr` came from `into_raw` and remains
        // allocated. Reading the header does not move or free the allocation.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        header.is_borrowed()
    }

    unsafe fn release_borrow(ptr: *mut std::ffi::c_void) {
        // SAFETY: `PrivateDataRef` only calls this for a live allocation whose
        // shared borrow was acquired by `borrow_ref`.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        header.release_borrow();
    }

    unsafe fn release_borrow_mut(ptr: *mut std::ffi::c_void) {
        // SAFETY: `PrivateDataMut` only calls this for a live allocation whose
        // exclusive borrow was acquired by `borrow_mut`.
        let header = unsafe { &*(ptr as *const PrivateDataHeader) };
        header.release_borrow_mut();
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
    pub(crate) constructor_methods: Vec<JSClassConstructorMethod>,
}

pub(crate) struct JSClassConstructorMethod {
    pub(crate) name: String,
    pub(crate) callback: JSObjectCallAsFunctionCallback,
    pub(crate) attributes: JSPropertyAttributes,
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

/// Borrowed JavaScriptCore callback context injected by callback macros.
///
/// This is a role marker for ergonomic macro signatures. It wraps the same
/// borrowed, non-owning context view as [`JSContext`] and does not retain or
/// release the underlying JavaScriptCore context.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct CallbackContext {
    inner: JSContext,
}

/// Function object for the active `#[callback]` invocation.
///
/// Use this marker when a callback needs the JavaScript function object that
/// JavaScriptCore invoked. Ordinary JavaScript object arguments should stay as
/// [`JSObject`] typed parameters.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct CallbackFunction {
    object: JSObject,
}

/// JavaScript `this` object for the active `#[callback]` invocation.
///
/// Use this marker when a callback needs its receiver. Ordinary JavaScript
/// object arguments should stay as [`JSObject`] typed parameters.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ThisObject {
    object: JSObject,
}

/// Constructor object for the active `#[constructor]` invocation.
///
/// Use this marker when a constructor callback needs the JavaScript constructor
/// function. Ordinary JavaScript object arguments should stay as [`JSObject`]
/// typed parameters.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ConstructorObject {
    object: JSObject,
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

impl CallbackContext {
    /// Return the borrowed JavaScriptCore context view.
    pub fn as_context(&self) -> &JSContext {
        &self.inner
    }

    /// Consume the marker and return the borrowed context view.
    pub fn into_context(self) -> JSContext {
        self.inner
    }
}

impl From<JSContext> for CallbackContext {
    fn from(inner: JSContext) -> Self {
        Self { inner }
    }
}

impl From<CallbackContext> for JSContext {
    fn from(context: CallbackContext) -> Self {
        context.inner
    }
}

impl Deref for CallbackContext {
    type Target = JSContext;

    fn deref(&self) -> &Self::Target {
        self.as_context()
    }
}

impl std::fmt::Debug for CallbackContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackContext").finish()
    }
}

impl CallbackFunction {
    /// Return the invoked JavaScript function object.
    pub fn as_object(&self) -> &JSObject {
        &self.object
    }

    /// Consume the marker and return the invoked JavaScript function object.
    pub fn into_object(self) -> JSObject {
        self.object
    }
}

impl From<JSObject> for CallbackFunction {
    fn from(object: JSObject) -> Self {
        Self { object }
    }
}

impl From<CallbackFunction> for JSObject {
    fn from(function: CallbackFunction) -> Self {
        function.object
    }
}

impl Deref for CallbackFunction {
    type Target = JSObject;

    fn deref(&self) -> &Self::Target {
        self.as_object()
    }
}

impl ThisObject {
    /// Return the JavaScript receiver object.
    pub fn as_object(&self) -> &JSObject {
        &self.object
    }

    /// Consume the marker and return the JavaScript receiver object.
    pub fn into_object(self) -> JSObject {
        self.object
    }
}

impl From<JSObject> for ThisObject {
    fn from(object: JSObject) -> Self {
        Self { object }
    }
}

impl From<ThisObject> for JSObject {
    fn from(this: ThisObject) -> Self {
        this.object
    }
}

impl Deref for ThisObject {
    type Target = JSObject;

    fn deref(&self) -> &Self::Target {
        self.as_object()
    }
}

impl ConstructorObject {
    /// Return the invoked JavaScript constructor object.
    pub fn as_object(&self) -> &JSObject {
        &self.object
    }

    /// Consume the marker and return the invoked JavaScript constructor object.
    pub fn into_object(self) -> JSObject {
        self.object
    }
}

impl From<JSObject> for ConstructorObject {
    fn from(object: JSObject) -> Self {
        Self { object }
    }
}

impl From<ConstructorObject> for JSObject {
    fn from(constructor: ConstructorObject) -> Self {
        constructor.object
    }
}

impl Deref for ConstructorObject {
    type Target = JSObject;

    fn deref(&self) -> &Self::Target {
        self.as_object()
    }
}

/// A JavaScript promise.
pub struct JSPromise {
    this: JSObject,
    resolver: JSPromiseResolvingFunctions,
    _not_send_or_sync: NotSendOrSync,
}

/// Preferred alias for JavaScript promise handles.
pub type Promise = JSPromise;

/// RAII guard for an installed unhandled-rejection callback.
///
/// Keep this guard alive for as long as the callback should remain rooted by
/// Rust. Dropping it releases the Rust protection count; JavaScriptCore's
/// global callback registration remains in place until the context is dropped
/// or the host installs another handler.
#[derive(Clone, Debug)]
pub struct UnhandledRejectionHandler {
    pub(crate) callback: ProtectedObject,
}

/// A JavaScript promise resolver.
#[derive(Debug)]
pub struct JSPromiseResolvingFunctions {
    resolve: JSObject,
    resolve_protection: ProtectedValue,
    reject: JSObject,
    reject_protection: ProtectedValue,
    _not_send_or_sync: NotSendOrSync,
}

/// Preferred alias for JavaScript promise resolver handles.
pub type PromiseResolver = JSPromiseResolvingFunctions;

/// A JavaScript value.
#[derive(Debug, Clone)]
pub struct JSValue {
    pub(crate) inner: JSValueRef,
    pub(crate) ctx: JSContextRef,
}

const STACK_VALUE_REF_CAPACITY: usize = 8;

#[inline]
pub(crate) fn with_raw_value_refs<R>(
    values: &[JSValue],
    f: impl FnOnce(usize, *const JSValueRef) -> R,
) -> R {
    if values.len() <= STACK_VALUE_REF_CAPACITY {
        let mut stack = [std::ptr::null(); STACK_VALUE_REF_CAPACITY];
        for (slot, value) in stack.iter_mut().zip(values) {
            *slot = value.inner;
        }
        f(values.len(), stack.as_ptr())
    } else {
        let refs: Vec<_> = values.iter().map(|value| value.inner).collect();
        f(refs.len(), refs.as_ptr())
    }
}

/// RAII guard for a JavaScript value protected from garbage collection.
///
/// Creating this guard calls `JSValueProtect`; dropping it calls
/// `JSValueUnprotect` exactly once for that protection count. The guarded value
/// remains context-affine and must not be used from another JavaScriptCore
/// thread.
#[derive(Debug)]
pub struct ProtectedValue {
    pub(crate) value: JSValue,
    pub(crate) _not_send_or_sync: NotSendOrSync,
}

/// RAII guard for a JavaScript object protected from garbage collection.
///
/// This is the object-typed counterpart to [`ProtectedValue`]. It keeps the
/// object alive while Rust stores it in callback tables, async completions, or
/// runtime state, and releases exactly one protection count on drop.
#[derive(Clone, Debug)]
pub struct ProtectedObject {
    object: JSObject,
    _protection: ProtectedValue,
}

/// A JavaScript class attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JSClassAttribute {
    /// Specifies that a class has no special attributes.
    #[default]
    None = kJSClassAttributeNone as isize,
    /// Specifies that a class should not automatically generate a shared prototype for its instance objects.
    /// Use it with [`JSClass::object_with_prototype`] or
    /// [`JSObject::set_prototype_checked`] to manage prototypes manually.
    NoAutomaticPrototype = kJSClassAttributeNoAutomaticPrototype as isize,
}

impl From<JSClassAttribute> for JSClassAttributes {
    fn from(value: JSClassAttribute) -> Self {
        value as JSClassAttributes
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    #[default]
    None = JSTypedArrayType_kJSTypedArrayTypeNone as isize,
    BigInt64Array = JSTypedArrayType_kJSTypedArrayTypeBigInt64Array as isize,
    BigUint64Array = JSTypedArrayType_kJSTypedArrayTypeBigUint64Array as isize,
}

impl From<JSTypedArrayType> for MJSTypedArrayType {
    fn from(value: JSTypedArrayType) -> Self {
        value as MJSTypedArrayType
    }
}

impl JSTypedArrayType {
    pub(crate) const fn element_size(self) -> Option<usize> {
        match self {
            JSTypedArrayType::Int8Array
            | JSTypedArrayType::Uint8Array
            | JSTypedArrayType::Uint8ClampedArray => Some(1),
            JSTypedArrayType::Int16Array | JSTypedArrayType::Uint16Array => Some(2),
            JSTypedArrayType::Int32Array
            | JSTypedArrayType::Uint32Array
            | JSTypedArrayType::Float32Array => Some(4),
            JSTypedArrayType::Float64Array
            | JSTypedArrayType::BigInt64Array
            | JSTypedArrayType::BigUint64Array => Some(8),
            JSTypedArrayType::ArrayBuffer | JSTypedArrayType::None => None,
        }
    }

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

/// A typed JavaScript property key accepted by the high-level object API.
///
/// This keeps string, value, and array-index keys explicit while letting callers
/// use one object API instead of choosing between several low-level C callback
/// shapes.
#[derive(Debug, Clone)]
pub enum PropertyKey {
    /// A JavaScript string property name.
    String(JSString),
    /// A JavaScript value that JavaScriptCore converts with `ToPropertyKey`.
    Value(JSValue),
    /// An array-index style property key.
    Index(u32),
}

impl PropertyKey {
    /// Creates a string property key.
    pub fn string(value: impl Into<JSString>) -> Self {
        Self::String(value.into())
    }

    /// Creates a property key from a JavaScript value.
    ///
    /// JavaScriptCore converts this value with `ToPropertyKey`, so symbols,
    /// strings, numbers, and objects with conversion hooks keep JavaScript
    /// semantics. Object APIs validate that the value belongs to the target
    /// object's context before crossing the C API.
    pub fn value(value: impl Into<JSValue>) -> Self {
        Self::Value(value.into())
    }

    /// Creates a JavaScript symbol property key in `ctx`.
    pub fn symbol(ctx: &JSContext, description: impl Into<JSString>) -> Self {
        Self::Value(JSValue::symbol(ctx, description))
    }

    /// Creates an array-index property key.
    pub fn index(value: u32) -> Self {
        Self::Index(value)
    }
}

impl From<JSString> for PropertyKey {
    fn from(value: JSString) -> Self {
        Self::String(value)
    }
}

impl From<&str> for PropertyKey {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for PropertyKey {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<&String> for PropertyKey {
    fn from(value: &String) -> Self {
        Self::String(value.as_str().into())
    }
}

impl From<JSValue> for PropertyKey {
    fn from(value: JSValue) -> Self {
        Self::Value(value)
    }
}

impl From<&JSValue> for PropertyKey {
    fn from(value: &JSValue) -> Self {
        Self::Value(value.clone())
    }
}

impl From<u32> for PropertyKey {
    fn from(value: u32) -> Self {
        Self::Index(value)
    }
}

/// A JavaScript string reference retained for use beyond a temporary call.
///
/// This handle releases the string automatically in `Drop`. Use
/// [`JSStringProtected::into_raw`] only when transferring the retained
/// reference to JavaScriptCore or another owner that will release it.
pub struct JSStringProtected(JSStringRef);

/// Deprecated misspelled alias for [`JSStringProtected`].
#[deprecated(
    since = "1.0.0",
    note = "use JSStringProtected; JSStringProctected will be removed after the 1.0 migration window"
)]
pub type JSStringProctected = JSStringProtected;

pub type JSResult<T> = Result<T, JSError>;

/// JavaScriptCore property attributes for a property definition.
#[derive(Debug, Clone, Copy)]
pub struct PropertyDescriptor {
    attributes: JSPropertyAttributes,
}

impl PropertyDescriptor {
    /// Creates a property descriptor from raw JavaScriptCore attribute bits.
    pub fn new(attributes: JSPropertyAttributes) -> Self {
        Self::from_raw_attributes(attributes)
    }

    /// Creates a property descriptor from raw JavaScriptCore attribute bits.
    pub fn from_raw_attributes(attributes: JSPropertyAttributes) -> Self {
        Self { attributes }
    }

    /// Creates a descriptor builder.
    pub fn builder() -> PropertyDescriptorBuilder {
        PropertyDescriptorBuilder::new()
    }

    /// Returns the raw JavaScriptCore property-attribute bitset.
    pub fn raw_attributes(&self) -> JSPropertyAttributes {
        self.attributes
    }

    /// Returns true when JavaScript assignment can write this property.
    pub fn is_writable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeReadOnly) == 0
    }

    /// Returns true when this descriptor marks the property as read-only.
    pub fn is_read_only(&self) -> bool {
        !self.is_writable()
    }

    /// Check if the property is enumerable
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperty#enumerable
    pub fn is_enumerable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeDontEnum) == 0
    }

    /// Returns true when this descriptor hides the property from enumeration.
    pub fn is_non_enumerable(&self) -> bool {
        !self.is_enumerable()
    }

    /// Returns true when the property can be reconfigured or deleted.
    pub fn is_configurable(&self) -> bool {
        (self.attributes & kJSPropertyAttributeDontDelete) == 0
    }

    /// Returns true when this descriptor prevents reconfiguration and deletion.
    pub fn is_non_configurable(&self) -> bool {
        !self.is_configurable()
    }
}

impl Default for PropertyDescriptor {
    fn default() -> Self {
        Self {
            attributes: kJSPropertyAttributeNone,
        }
    }
}

/// Builder for JavaScriptCore property attributes.
pub struct PropertyDescriptorBuilder {
    attributes: JSPropertyAttributes,
}

impl PropertyDescriptorBuilder {
    pub fn new() -> Self {
        Self {
            attributes: kJSPropertyAttributeNone,
        }
    }

    pub fn writable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeReadOnly, value)
    }

    /// Marks the property as read-only.
    pub fn read_only(self) -> Self {
        self.writable(false)
    }

    pub fn enumerable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeDontEnum, value)
    }

    /// Hides the property from enumeration.
    pub fn non_enumerable(self) -> Self {
        self.enumerable(false)
    }

    pub fn configurable(self, value: bool) -> Self {
        self.set_attribute(kJSPropertyAttributeDontDelete, value)
    }

    /// Prevents property reconfiguration and deletion.
    pub fn non_configurable(self) -> Self {
        self.configurable(false)
    }

    fn set_attribute(mut self, attribute: JSPropertyAttributes, value: bool) -> Self {
        if value {
            self.attributes &= !attribute;
        } else {
            self.attributes |= attribute;
        }
        self
    }

    pub fn build(self) -> PropertyDescriptor {
        PropertyDescriptor {
            attributes: self.attributes,
        }
    }
}

impl Default for PropertyDescriptorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_value_refs_preserve_order_for_small_and_large_arguments() {
        let ctx = JSContext::new();
        let values: Vec<_> = (0..9)
            .map(|index| JSValue::number(&ctx, index as f64))
            .collect();

        with_raw_value_refs(&values[..8], |argument_count, arguments| {
            assert_eq!(argument_count, 8);
            let arguments =
                // SAFETY: `arguments` points to `argument_count` entries for
                // the duration of this closure.
                unsafe { std::slice::from_raw_parts(arguments, argument_count) };
            assert_eq!(arguments[0], values[0].inner);
            assert_eq!(arguments[7], values[7].inner);
        });

        with_raw_value_refs(&values, |argument_count, arguments| {
            assert_eq!(argument_count, values.len());
            let arguments =
                // SAFETY: `arguments` points to `argument_count` entries for
                // the duration of this closure.
                unsafe { std::slice::from_raw_parts(arguments, argument_count) };
            assert_eq!(arguments[0], values[0].inner);
            assert_eq!(arguments[8], values[8].inner);
        });
    }

    #[test]
    fn test_property_descriptor_builder() {
        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder
            .writable(true)
            .enumerable(true)
            .configurable(true)
            .build();
        assert!(descriptor.is_writable());
        assert!(descriptor.is_enumerable());
        assert!(descriptor.is_configurable());

        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder
            .writable(false)
            .enumerable(false)
            .configurable(false)
            .build();
        assert!(descriptor.is_read_only());
        assert!(descriptor.is_non_enumerable());
        assert!(descriptor.is_non_configurable());

        let descriptor = PropertyDescriptor::builder()
            .non_enumerable()
            .read_only()
            .non_configurable()
            .build();
        assert!(descriptor.is_read_only());
        assert!(descriptor.is_non_enumerable());
        assert!(descriptor.is_non_configurable());

        let builder = PropertyDescriptorBuilder::new();
        let descriptor = builder.build();
        assert!(descriptor.is_writable());
        assert!(descriptor.is_enumerable());
        assert!(descriptor.is_configurable());

        assert_eq!(
            PropertyDescriptor::from_raw_attributes(descriptor.raw_attributes())
                .raw_attributes(),
            descriptor.raw_attributes()
        );
    }
}
