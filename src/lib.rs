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

/// A JavaScript context.
pub struct JSContext {
    pub(crate) inner: JSGlobalContextRef,
}

pub type PrivateData = *mut ::std::os::raw::c_void;

/// A JavaScript execution context group.
pub struct JSContextGroup {
    context_group: JSContextGroupRef,
}

/// A JavaScript class.
pub struct JSClass {
    // pub(crate) ctx: JSContextRef,
    pub(crate) inner: JSClassRef,
    pub(crate) name: String,
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
pub struct JSStringRetain(JSStringRef);

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
