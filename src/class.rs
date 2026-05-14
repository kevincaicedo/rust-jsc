use std::any::TypeId;
use std::ffi::CString;
use std::fmt;
use std::ptr;

use crate::{
    self as rust_jsc, finalize, IntoJSValue, JSClass, JSClassConstructorMethod,
    JSContext, JSError, JSFunction, JSObject, JSResult, JSValue, PrivateData,
    PrivateDataSetStatus, PrivateDataWrapper, PropertyDescriptor, TryFromJSValue,
};
use rust_jsc_sys::{
    kJSClassDefinitionEmpty, JSClassCreate, JSClassDefinition, JSClassRelease,
    JSContextRef, JSObjectCallAsConstructorCallback, JSObjectCallAsFunctionCallback,
    JSObjectConvertToTypeCallback, JSObjectDeletePropertyCallback,
    JSObjectFinalizeCallback, JSObjectGetPropertyCallback,
    JSObjectGetPropertyNamesCallback, JSObjectHasInstanceCallback,
    JSObjectHasPropertyCallback, JSObjectInitializeCallback, JSObjectMake, JSObjectRef,
    JSObjectSetPrivate, JSObjectSetPropertyCallback, JSStaticFunction, JSStaticValue,
    JSStringRef, JSValueRef,
};

#[derive(Debug)]
pub enum ClassError {
    NameContainsNul {
        field: &'static str,
        position: usize,
    },
    MissingCallback {
        field: &'static str,
    },
    CreateFailed,
}

impl fmt::Display for ClassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameContainsNul { field, position } => {
                write!(
                    formatter,
                    "{field} contains an interior NUL byte at offset {position}"
                )
            }
            Self::MissingCallback { field } => {
                write!(formatter, "{field} requires a callback")
            }
            Self::CreateFailed => {
                formatter.write_str("failed to create JavaScript class")
            }
        }
    }
}

impl std::error::Error for ClassError {}

/// Static typed accessor adapter for [`JSClassBuilder::typed_accessor`].
///
/// Implement this trait on a zero-sized marker type when a class property has a
/// known Rust value type. Rust-JSC provides the JavaScriptCore getter and
/// setter callbacks, catches panics before they cross the C ABI, and maps
/// conversion or accessor errors into JavaScript exceptions.
pub trait JSClassAccessor {
    type Value: TryFromJSValue + IntoJSValue;

    /// Reads the property value.
    ///
    /// # Errors
    /// Return a [`JSError`] to throw a JavaScript exception from the getter.
    fn get(ctx: JSContext, object: JSObject) -> JSResult<Self::Value>;

    /// Writes the property value.
    ///
    /// The default implementation throws a JavaScript error, making the typed
    /// accessor read-only unless the marker type overrides this method.
    ///
    /// # Errors
    /// Return a [`JSError`] to throw a JavaScript exception from the setter.
    fn set(ctx: JSContext, _object: JSObject, _value: Self::Value) -> JSResult<()> {
        Err(JSError::from_message(&ctx, "property is read-only"))
    }
}

pub struct JSClassBuilder {
    definition: JSClassDefinition,
    class_name: CString,
    name: String,
    static_value_names: Vec<CString>,
    static_values: Vec<JSStaticValue>,
    static_function_names: Vec<CString>,
    static_functions: Vec<JSStaticFunction>,
    constructor_methods: Vec<JSClassConstructorMethod>,
}

impl JSClassBuilder {
    pub fn new(name: &str) -> Self {
        Self::try_new(name).expect("class name must not contain interior NUL bytes")
    }

    pub fn try_new(name: &str) -> Result<Self, ClassError> {
        // SAFETY: `kJSClassDefinitionEmpty` is JavaScriptCore's immutable
        // template value for initializing a `JSClassDefinition` by copy.
        let mut definition = unsafe { kJSClassDefinitionEmpty };

        let class_name =
            CString::new(name).map_err(|error| ClassError::NameContainsNul {
                field: "class name",
                position: error.nul_position(),
            })?;
        definition.className = class_name.as_ptr();
        Ok(Self {
            definition,
            class_name,
            name: name.to_string(),
            static_value_names: Vec::new(),
            static_values: Vec::new(),
            static_function_names: Vec::new(),
            static_functions: Vec::new(),
            constructor_methods: Vec::new(),
        })
    }

    pub fn set_version(mut self, version: u32) -> Self {
        self.definition.version = version as i32;
        self
    }

    pub fn set_attributes(mut self, attributes: u32) -> Self {
        self.definition.attributes = attributes;
        self
    }

    pub fn parent_class(mut self, parent_class: &JSClass) -> Self {
        self.definition.parentClass = parent_class.inner;
        self
    }

    pub fn set_initialize(mut self, initialize: JSObjectInitializeCallback) -> Self {
        self.definition.initialize = initialize;
        self
    }

    /// Sets the finalize callback for the class.
    ///
    /// **WARNING**: If you set a custom finalize callback, Rust-JSC will NOT automatically
    /// free the attached Rust data (`TypedData<T>`). This will cause a memory leak unless
    /// you manually free the data.
    ///
    /// **Recommended Alternative**: Instead of using a custom finalize callback, implement
    /// the `Drop` trait on your Rust data type `T`. Rust-JSC's default finalizer will
    /// automatically drop your data and free the memory when the JavaScript object is
    /// garbage collected.
    pub fn set_finalize(mut self, finalize: JSObjectFinalizeCallback) -> Self {
        self.definition.finalize = finalize;
        self
    }

    pub fn has_property(mut self, has_property: JSObjectHasPropertyCallback) -> Self {
        self.definition.hasProperty = has_property;
        self
    }

    pub fn get_property(mut self, get_property: JSObjectGetPropertyCallback) -> Self {
        self.definition.getProperty = get_property;
        self
    }

    pub fn set_property(mut self, set_property: JSObjectSetPropertyCallback) -> Self {
        self.definition.setProperty = set_property;
        self
    }

    pub fn delete_property(
        mut self,
        delete_property: JSObjectDeletePropertyCallback,
    ) -> Self {
        self.definition.deleteProperty = delete_property;
        self
    }

    pub fn get_property_names(
        mut self,
        get_property_names: JSObjectGetPropertyNamesCallback,
    ) -> Self {
        self.definition.getPropertyNames = get_property_names;
        self
    }

    pub fn call_as_function(
        mut self,
        call_as_function: JSObjectCallAsFunctionCallback,
    ) -> Self {
        self.definition.callAsFunction = call_as_function;
        self
    }

    pub fn call_as_constructor(
        mut self,
        call_as_constructor: JSObjectCallAsConstructorCallback,
    ) -> Self {
        self.definition.callAsConstructor = call_as_constructor;
        self
    }

    pub fn has_instance(mut self, has_instance: JSObjectHasInstanceCallback) -> Self {
        self.definition.hasInstance = has_instance;
        self
    }

    pub fn convert_to_type(
        mut self,
        convert_to_type: JSObjectConvertToTypeCallback,
    ) -> Self {
        self.definition.convertToType = convert_to_type;
        self
    }

    /// Adds a prototype method using JavaScriptCore's static function table.
    ///
    /// Static functions are copied by `JSClassCreate` and, unless
    /// `NoAutomaticPrototype` is set, are installed on the shared prototype
    /// rather than copied onto each instance.
    pub fn method(
        self,
        name: &str,
        callback: JSObjectCallAsFunctionCallback,
    ) -> Result<Self, ClassError> {
        self.method_with_attributes(name, callback, PropertyDescriptor::default())
    }

    /// Adds a prototype method with explicit JavaScript property attributes.
    pub fn method_with_attributes(
        mut self,
        name: &str,
        callback: JSObjectCallAsFunctionCallback,
        attributes: PropertyDescriptor,
    ) -> Result<Self, ClassError> {
        if callback.is_none() {
            return Err(ClassError::MissingCallback {
                field: "prototype method",
            });
        }

        let name = CString::new(name).map_err(|error| ClassError::NameContainsNul {
            field: "prototype method name",
            position: error.nul_position(),
        })?;
        let name_ptr = name.as_ptr();
        self.static_function_names.push(name);
        self.static_functions.push(JSStaticFunction {
            name: name_ptr,
            callAsFunction: callback,
            attributes: attributes.raw_attributes(),
        });

        Ok(self)
    }

    /// Adds a static method to the constructor object installed by
    /// [`JSClass::register`].
    ///
    /// Unlike [`JSClassBuilder::method`], this does not use JavaScriptCore's
    /// static function table because that table describes instance prototype
    /// methods. Constructor methods are created when the class is registered in
    /// a context and are installed as own properties of the registered class
    /// object.
    pub fn constructor_method(
        self,
        name: &str,
        callback: JSObjectCallAsFunctionCallback,
    ) -> Result<Self, ClassError> {
        self.constructor_method_with_attributes(
            name,
            callback,
            PropertyDescriptor::default(),
        )
    }

    /// Adds a static constructor method with explicit JavaScript property
    /// attributes.
    pub fn constructor_method_with_attributes(
        mut self,
        name: &str,
        callback: JSObjectCallAsFunctionCallback,
        attributes: PropertyDescriptor,
    ) -> Result<Self, ClassError> {
        if callback.is_none() {
            return Err(ClassError::MissingCallback {
                field: "constructor method",
            });
        }

        CString::new(name).map_err(|error| ClassError::NameContainsNul {
            field: "constructor method name",
            position: error.nul_position(),
        })?;
        self.constructor_methods.push(JSClassConstructorMethod {
            name: name.to_owned(),
            callback,
            attributes: attributes.raw_attributes(),
        });

        Ok(self)
    }

    /// Adds a statically declared value property.
    ///
    /// JavaScriptCore services these properties directly before falling back to
    /// dynamic property callbacks, so use them for known accessors.
    pub fn accessor(
        self,
        name: &str,
        get_property: JSObjectGetPropertyCallback,
        set_property: JSObjectSetPropertyCallback,
    ) -> Result<Self, ClassError> {
        self.accessor_with_attributes(
            name,
            get_property,
            set_property,
            PropertyDescriptor::default(),
        )
    }

    /// Adds a statically declared value property with explicit attributes.
    pub fn accessor_with_attributes(
        mut self,
        name: &str,
        get_property: JSObjectGetPropertyCallback,
        set_property: JSObjectSetPropertyCallback,
        attributes: PropertyDescriptor,
    ) -> Result<Self, ClassError> {
        if get_property.is_none() && set_property.is_none() {
            return Err(ClassError::MissingCallback { field: "accessor" });
        }

        let name = CString::new(name).map_err(|error| ClassError::NameContainsNul {
            field: "accessor name",
            position: error.nul_position(),
        })?;
        let name_ptr = name.as_ptr();
        self.static_value_names.push(name);
        self.static_values.push(JSStaticValue {
            name: name_ptr,
            getProperty: get_property,
            setProperty: set_property,
            attributes: attributes.raw_attributes(),
        });

        Ok(self)
    }

    /// Adds a typed static value accessor.
    ///
    /// The marker type `A` supplies Rust getter/setter behavior through
    /// [`JSClassAccessor`]. Values are converted with [`TryFromJSValue`] and
    /// [`IntoJSValue`], and panics are converted into JavaScript exceptions
    /// before returning to JavaScriptCore.
    pub fn typed_accessor<A>(self, name: &str) -> Result<Self, ClassError>
    where
        A: JSClassAccessor + 'static,
    {
        self.typed_accessor_with_attributes::<A>(name, PropertyDescriptor::default())
    }

    /// Adds a typed static value accessor with explicit JavaScript property
    /// attributes.
    pub fn typed_accessor_with_attributes<A>(
        self,
        name: &str,
        attributes: PropertyDescriptor,
    ) -> Result<Self, ClassError>
    where
        A: JSClassAccessor + 'static,
    {
        self.accessor_with_attributes(
            name,
            Some(Self::typed_get_property::<A>),
            Some(Self::typed_set_property::<A>),
            attributes,
        )
    }

    unsafe extern "C" fn typed_get_property<A>(
        ctx: JSContextRef,
        object: JSObjectRef,
        _property_name: JSStringRef,
        exception: *mut JSValueRef,
    ) -> JSValueRef
    where
        A: JSClassAccessor + 'static,
    {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // SAFETY: JavaScriptCore invokes static value callbacks with a live
            // context for the duration of the callback. The borrowed view does
            // not retain or release the context.
            let ctx = unsafe { JSContext::borrowed(ctx) };
            let object = JSObject::from_ref(object, ctx.inner);
            A::get(ctx, object).and_then(|value| value.into_js_value(&ctx))
        }));

        match result {
            Ok(Ok(value)) => value.inner,
            Ok(Err(error)) => {
                Self::write_accessor_exception(exception, error);
                ptr::null_mut()
            }
            Err(_) => {
                // SAFETY: JavaScriptCore provided a live callback context. This
                // borrowed view is used only to construct the exception object.
                let ctx = unsafe { JSContext::borrowed(ctx) };
                Self::write_accessor_exception(
                    exception,
                    JSError::from_message(&ctx, "Rust class accessor getter panicked"),
                );
                ptr::null_mut()
            }
        }
    }

    unsafe extern "C" fn typed_set_property<A>(
        ctx: JSContextRef,
        object: JSObjectRef,
        _property_name: JSStringRef,
        value: JSValueRef,
        exception: *mut JSValueRef,
    ) -> bool
    where
        A: JSClassAccessor + 'static,
    {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // SAFETY: JavaScriptCore invokes static value callbacks with live
            // context, object, and value handles for the duration of the
            // callback. The borrowed context view does not retain or release.
            let ctx = unsafe { JSContext::borrowed(ctx) };
            let object = JSObject::from_ref(object, ctx.inner);
            let value = JSValue::new(value, ctx.inner);
            let value = A::Value::try_from_js_value(&value)?;
            A::set(ctx, object, value)
        }));

        match result {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                Self::write_accessor_exception(exception, error);
                false
            }
            Err(_) => {
                // SAFETY: JavaScriptCore provided a live callback context. This
                // borrowed view is used only to construct the exception object.
                let ctx = unsafe { JSContext::borrowed(ctx) };
                Self::write_accessor_exception(
                    exception,
                    JSError::from_message(&ctx, "Rust class accessor setter panicked"),
                );
                false
            }
        }
    }

    fn write_accessor_exception(exception: *mut JSValueRef, error: JSError) {
        if exception.is_null() {
            return;
        }

        let value: JSValue = error.into();
        // SAFETY: JavaScriptCore owns the non-null exception out-pointer for
        // the duration of the accessor callback.
        unsafe {
            *exception = value.inner;
        }
    }

    #[finalize]
    fn finalize_callback<T: 'static>(data_ptr: PrivateData) {
        // SAFETY: JavaScriptCore calls this finalizer with the private-data
        // pointer installed for this class. `build::<T>` wires the finalizer to
        // the same `T`, and `drop_raw` tolerates null/foreign state.
        let _ = unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
    }

    pub fn build<T: 'static>(mut self) -> Result<JSClass, ClassError> {
        if self.definition.finalize.is_none() && TypeId::of::<T>() != TypeId::of::<()>() {
            self.definition.finalize = Some(Self::finalize_callback::<T>);
        }
        self.definition.className = self.class_name.as_ptr();
        if !self.static_values.is_empty() {
            self.static_values.push(JSStaticValue {
                name: ptr::null(),
                getProperty: None,
                setProperty: None,
                attributes: 0,
            });
            self.definition.staticValues = self.static_values.as_ptr();
        }
        if !self.static_functions.is_empty() {
            self.static_functions.push(JSStaticFunction {
                name: ptr::null(),
                callAsFunction: None,
                attributes: 0,
            });
            self.definition.staticFunctions = self.static_functions.as_ptr();
        }

        // SAFETY: `definition` points to a stack-local JSClassDefinition whose
        // class name and static member names are backed by this builder until
        // `JSClassCreate` returns. JavaScriptCore copies those strings and
        // callback entries into the created class.
        let class = unsafe { JSClassCreate(&self.definition) };
        if class.is_null() {
            return Err(ClassError::CreateFailed);
        }

        Ok(JSClass {
            inner: class,
            name: self.name,
            type_id: TypeId::of::<T>(),
            constructor_methods: self.constructor_methods,
        })
    }
}

impl JSClass {
    /// Creates a new class builder.
    ///
    /// # Arguments
    /// - `name`: The name of the class.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::{JSClass, JSClassBuilder};
    ///
    /// let builder = JSClass::builder("Test");
    ///
    /// let class = builder
    ///     .set_version(1)
    ///     .set_attributes(JSClassAttribute::None.into())
    ///     .set_initialize(None)
    ///     .build()
    ///     .expect("Failed to create class");
    /// ```
    ///
    /// With constructor:
    ///
    /// ```rust,ignore
    /// use rust_jsc_macros::constructor;
    /// use rust_jsc::{JSClass, JSClassBuilder, JSClassAttribute, JSResult, JSValue, JSObject, JSContext};
    ///
    /// #[constructor]
    /// fn constructor(
    ///    _ctx: JSContext,
    ///   this: JSObject,
    ///  _arguments: &[JSValue],
    /// ) -> JSResult<JSValue> {
    ///    let value = JSValue::string(&_ctx, "John");
    ///   this.set_property(&"name".into(), &value, Default::default())
    ///      .unwrap();
    ///
    ///   Ok(this.into())
    /// }
    ///
    /// let builder = JSClass::builder("Test");
    ///
    /// let class = builder
    ///    .set_version(1)
    ///    .set_attributes(JSClassAttribute::None.into())
    ///    .set_initialize(None)
    ///    .set_finalize(None)
    ///    .has_property(None)
    ///    .get_property(None)
    ///    .set_property(None)
    ///    .delete_property(None)
    ///    .get_property_names(None)
    ///    .call_as_function(None)
    ///    .call_as_constructor(Some(constructor))
    ///    .has_instance(None)
    ///    .convert_to_type(None)
    ///    .build()
    ///    .expect("Failed to create class");
    /// ```
    ///
    /// # Returns
    /// A new class builder.
    pub fn builder(name: &str) -> JSClassBuilder {
        JSClassBuilder::new(name)
    }

    /// Creates a new class builder without panicking on an invalid class name.
    ///
    /// Use this in embedders that accept class names from outside Rust source
    /// code. [`JSClass::builder`] remains as the concise path for static names.
    pub fn try_builder(name: &str) -> Result<JSClassBuilder, ClassError> {
        JSClassBuilder::try_new(name)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates a new object of the class.
    /// The object will be created in the given context.
    /// The object will have the given data associated with it.
    /// The data will be passed to the initialize callback.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to create the object in.
    /// - `data`: The data to associate with the object.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSClass, JSContext};
    ///
    /// let ctx = JSContext::new();
    /// let class = JSClass::builder("Test")
    ///    .set_version(1)
    ///    .build::<i32>()
    ///    .unwrap();
    ///
    /// let object = class.object::<i32>(&ctx, Some(42));
    /// ```
    ///
    /// # Returns
    /// A new object of the class.
    pub fn object<T: 'static>(&self, ctx: &JSContext, data: Option<T>) -> JSObject {
        assert!(
            self.type_id == TypeId::of::<T>(),
            "Data type does not match class type"
        );

        let data_ptr = if let Some(data) = data {
            PrivateDataWrapper::into_raw(data)
        } else {
            std::ptr::null_mut()
        };

        // SAFETY: `ctx.inner` and `self.inner` are live handles. `data_ptr` is
        // either null or a Rust allocation transferred into JavaScriptCore's
        // private slot and released by this class finalizer.
        let inner = unsafe { JSObjectMake(ctx.inner, self.inner, data_ptr) };
        JSObject::from_ref(inner, ctx.inner)
    }

    /// Creates a new object of this class and assigns an explicit prototype.
    ///
    /// Use this when the class is configured with
    /// [`JSClassAttribute::NoAutomaticPrototype`](crate::JSClassAttribute::NoAutomaticPrototype)
    /// or when an embedder owns prototype objects outside JavaScriptCore's
    /// automatic class prototype.
    ///
    /// # Errors
    /// Returns a [`JSError`] if `prototype` belongs to a different JavaScript
    /// context than `ctx`.
    ///
    /// # Panics
    /// Panics if `T` does not match the type used to build this class.
    pub fn object_with_prototype<T: 'static>(
        &self,
        ctx: &JSContext,
        data: Option<T>,
        prototype: &JSObject,
    ) -> JSResult<JSObject> {
        if prototype.value.ctx != ctx.inner {
            return Err(JSError::from_message(
                ctx,
                "prototype object belongs to a different JavaScript context",
            ));
        }

        let object = self.object(ctx, data);
        object.set_prototype(prototype);
        Ok(object)
    }

    /// Stores private data into an empty private-data slot for an object of this
    /// class.
    ///
    /// This is the safe path for attaching private data after object creation:
    /// it checks that `T` matches the class data type, verifies that `object`
    /// is an instance of this class, and refuses to replace existing private
    /// data. Use the lower-level unsafe object APIs only when you explicitly
    /// own all references into the existing private data.
    pub fn set_object_private_data<T: 'static>(
        &self,
        object: &JSObject,
        data: T,
    ) -> JSResult<PrivateDataSetStatus> {
        if self.type_id != TypeId::of::<T>() {
            // SAFETY: `object` carries a borrowed JavaScriptCore context
            // pointer. This creates a non-owning view for error construction
            // and does not retain or release the context.
            let ctx = unsafe { JSContext::borrowed(object.value.ctx) };
            return Err(JSError::from_message(
                &ctx,
                "private data type does not match class type",
            ));
        }

        if !object.is_object_of_class(self)? {
            return Ok(PrivateDataSetStatus::Unsupported);
        }

        if object.get_private_data_ptr().is_some() {
            return Ok(PrivateDataSetStatus::AlreadySet);
        }

        let data_ptr = PrivateDataWrapper::into_raw(data);
        // SAFETY: `object` was verified to be an instance of this class, and
        // `T` matches the class finalizer type. The slot is empty, so this
        // method does not replace or invalidate existing private-data refs.
        let success = unsafe { JSObjectSetPrivate(object.inner, data_ptr) };
        if !success {
            // SAFETY: `data_ptr` was allocated above and was not accepted by
            // JavaScriptCore, so Rust must free it to avoid leaking.
            unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
            return Ok(PrivateDataSetStatus::Unsupported);
        }

        Ok(PrivateDataSetStatus::Set)
    }

    fn object_empty(&self, ctx: &JSContext) -> JSObject {
        // SAFETY: `ctx.inner` and `self.inner` are live handles; no private data
        // is attached for this constructor/class object.
        let inner = unsafe { JSObjectMake(ctx.inner, self.inner, std::ptr::null_mut()) };
        JSObject::from_ref(inner, ctx.inner)
    }

    /// Creates the class object used for registration and installs constructor
    /// static methods on it.
    ///
    /// This is useful when embedders need to publish the class under a custom
    /// module namespace instead of directly on the global object.
    pub fn constructor_object(&self, ctx: &JSContext) -> JSResult<JSObject> {
        let object = self.object_empty(ctx);
        self.install_constructor_methods(ctx, &object)?;
        Ok(object)
    }

    fn install_constructor_methods(
        &self,
        ctx: &JSContext,
        constructor: &JSObject,
    ) -> JSResult<()> {
        for method in &self.constructor_methods {
            let function =
                JSFunction::callback(ctx, Some(method.name.as_str()), method.callback);
            constructor.set_property(
                method.name.as_str(),
                &function,
                PropertyDescriptor::from_raw_attributes(method.attributes),
            )?;
        }

        Ok(())
    }

    /// Registers the class in the global object.
    /// This will make the class available in JavaScript.
    /// The class will be available as a constructor function.
    /// The class name will be the same as the class name in Rust.
    ///
    /// # Arguments
    /// - `ctx`: The JavaScript context to register the class in.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::{JSClass, JSContext, JSClassAttribute};
    ///
    /// let ctx = JSContext::new();
    /// let class = JSClass::builder("Test")
    ///     .set_version(1)
    ///     .set_attributes(JSClassAttribute::None.into())
    ///     .set_initialize(None)
    ///     .set_finalize(None)
    ///     .has_property(None)
    ///     .get_property(None)
    ///     .set_property(None)
    ///     .delete_property(None)
    ///     .get_property_names(None)
    ///     .call_as_function(None)
    ///     .call_as_constructor(None)
    ///     .has_instance(None)
    ///     .convert_to_type(None)
    ///     .build::<()>()
    ///     .unwrap();
    ///
    /// class.register(&ctx).unwrap();
    /// ```
    ///
    /// # Errors
    /// If an error occurs while registering the class.
    pub fn register(&self, ctx: &JSContext) -> JSResult<()> {
        let constructor = self.constructor_object(ctx)?;
        ctx.global_object()
            .set_property(self.name(), &constructor, Default::default())
    }
}

impl Drop for JSClass {
    fn drop(&mut self) {
        // SAFETY: `self.inner` is the owned `JSClassRef` created by
        // `JSClassCreate` and is released exactly once from `Drop`.
        unsafe { JSClassRelease(self.inner) };
    }
}

#[cfg(test)]
mod tests {
    use super::{ClassError, JSClassAccessor, JSClassBuilder};
    use crate::{self as rust_jsc, PrivateData, PrivateDataSetStatus};
    use rust_jsc_macros::{callback, constructor, finalize, has_instance, initialize};
    use rust_jsc_sys::{JSContextRef, JSObjectRef, JSObjectSetPrivate};
    use std::ffi::c_void;

    use crate::{JSClass, JSClassAttribute, JSContext, JSObject, JSResult, JSValue};

    fn expect_class_error<T>(result: Result<T, ClassError>) -> ClassError {
        match result {
            Ok(_) => panic!("expected class builder error"),
            Err(error) => error,
        }
    }

    #[test]
    fn test_class_builder() {
        #[constructor]
        fn constructor(
            _ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let value = JSValue::string(&_ctx, "John");
            this.set_property("name", &value, Default::default())
                .unwrap();
            Ok(this.into())
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("Test")
            .set_version(1)
            .set_attributes(JSClassAttribute::None.into())
            .set_initialize(None)
            .set_finalize(None)
            .has_property(None)
            .get_property(None)
            .set_property(None)
            .delete_property(None)
            .get_property_names(None)
            .call_as_function(None)
            .call_as_constructor(Some(constructor))
            .has_instance(None)
            .convert_to_type(None)
            .build::<isize>()
            .unwrap();

        let object = class.object::<isize>(&ctx, Some(42));

        ctx.global_object()
            .set_property("Test", &object, Default::default())
            .unwrap();
        let result_object = ctx
            .evaluate_script("const obj = new Test(); obj", None)
            .unwrap();

        assert!(result_object.is_object_of_class(&class).unwrap());
        assert!(object.is_object());
        let object = object.as_object().unwrap();
        assert!(object.has_property("name"));
        assert_eq!(
            object.get_property("name").unwrap(),
            JSValue::string(&ctx, "John")
        );
    }

    #[test]
    fn test_class_register() {
        #[constructor]
        fn constructor(
            _ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let value = JSValue::string(&_ctx, "John");
            this.set_property("name", &value, Default::default())
                .unwrap();
            Ok(this.into())
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("Test")
            .set_version(1)
            .set_attributes(JSClassAttribute::None.into())
            .set_initialize(None)
            .set_finalize(None)
            .has_property(None)
            .get_property(None)
            .set_property(None)
            .delete_property(None)
            .get_property_names(None)
            .call_as_function(None)
            .call_as_constructor(Some(constructor))
            .has_instance(None)
            .convert_to_type(None)
            .build::<()>()
            .unwrap();

        class.register(&ctx).unwrap();
        let result_object = ctx
            .evaluate_script("const obj = new Test(); obj", None)
            .unwrap();

        assert!(result_object.is_object_of_class(&class).unwrap());
    }

    #[test]
    fn test_class_without_constructor() {
        let ctx = JSContext::new();
        let class = JSClass::builder("Test")
            .set_version(1)
            .set_attributes(JSClassAttribute::None.into())
            .set_initialize(None)
            .set_finalize(None)
            .has_property(None)
            .get_property(None)
            .set_property(None)
            .delete_property(None)
            .get_property_names(None)
            .call_as_function(None)
            .call_as_constructor(None)
            .has_instance(None)
            .convert_to_type(None)
            .build::<()>()
            .unwrap();

        class.register(&ctx).unwrap();
        let result = ctx.evaluate_script("const obj = new Test(); obj", None);

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.name().unwrap(), "TypeError");
    }

    #[test]
    fn test_class_try_builder_rejects_interior_nul_name() {
        let error = expect_class_error(JSClass::try_builder("Bad\0Name"));
        assert!(matches!(
            error,
            ClassError::NameContainsNul {
                field: "class name",
                position: 3
            }
        ));
    }

    #[test]
    fn test_class_builder_rejects_invalid_method_configuration() {
        #[callback]
        fn method(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            Ok(JSValue::undefined(&ctx))
        }

        let error = expect_class_error(
            JSClass::try_builder("InvalidMethod")
                .unwrap()
                .method("bad\0method", Some(method)),
        );
        assert!(matches!(
            error,
            ClassError::NameContainsNul {
                field: "prototype method name",
                position: 3
            }
        ));

        let error = expect_class_error(
            JSClass::try_builder("MissingMethod")
                .unwrap()
                .method("missing", None),
        );
        assert!(matches!(
            error,
            ClassError::MissingCallback {
                field: "prototype method"
            }
        ));

        let error = expect_class_error(
            JSClass::try_builder("InvalidConstructorMethod")
                .unwrap()
                .constructor_method("bad\0method", Some(method)),
        );
        assert!(matches!(
            error,
            ClassError::NameContainsNul {
                field: "constructor method name",
                position: 3
            }
        ));

        let error = expect_class_error(
            JSClass::try_builder("MissingConstructorMethod")
                .unwrap()
                .constructor_method("missing", None),
        );
        assert!(matches!(
            error,
            ClassError::MissingCallback {
                field: "constructor method"
            }
        ));

        let error = expect_class_error(
            JSClass::try_builder("InvalidAccessor").unwrap().accessor(
                "bad\0accessor",
                None,
                None,
            ),
        );
        assert!(matches!(
            error,
            ClassError::MissingCallback { field: "accessor" }
        ));
    }

    #[test]
    fn test_class_builder_method_installs_shared_prototype_function() {
        #[constructor]
        fn constructor(
            ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            this.set_property("value", &JSValue::number(&ctx, 1.0), Default::default())?;
            Ok(this.into())
        }

        #[callback]
        fn increment(
            ctx: JSContext,
            _function: JSObject,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let next = this.get_property("value")?.as_number()? + 1.0;
            let value = JSValue::number(&ctx, next);
            this.set_property("value", &value, Default::default())?;
            Ok(value)
        }

        let ctx = JSContext::new();
        let class = JSClass::try_builder("Counter")
            .unwrap()
            .method("increment", Some(increment))
            .unwrap()
            .call_as_constructor(Some(constructor))
            .build::<()>()
            .unwrap();

        class.register(&ctx).unwrap();
        let result = ctx
            .evaluate_script(
                r#"
                const counter = new Counter();
                counter.increment();
                counter.value;
                "#,
                None,
            )
            .unwrap();

        assert_eq!(result.as_number().unwrap(), 2.0);
        assert!(ctx
            .evaluate_script(
                "Object.prototype.hasOwnProperty.call(Object.getPrototypeOf(counter), 'increment')",
                None,
            )
            .unwrap()
            .as_boolean());
    }

    #[test]
    fn test_class_builder_constructor_method_installs_on_registered_class() {
        #[callback]
        fn version(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            Ok(JSValue::string(&ctx, "1.0"))
        }

        let ctx = JSContext::new();
        let class = JSClass::try_builder("StaticTool")
            .unwrap()
            .constructor_method("version", Some(version))
            .unwrap()
            .build::<()>()
            .unwrap();

        class.register(&ctx).unwrap();

        assert_eq!(
            ctx.evaluate_script("StaticTool.version()", None)
                .unwrap()
                .as_string()
                .unwrap(),
            "1.0"
        );
        assert!(ctx
            .evaluate_script(
                "Object.prototype.hasOwnProperty.call(StaticTool, 'version')",
                None,
            )
            .unwrap()
            .as_boolean());
    }

    #[test]
    fn test_class_builder_accessor_reads_and_writes_private_data() {
        unsafe extern "C" fn get_count(
            ctx: rust_jsc::internal::JSContextRef,
            object: rust_jsc::internal::JSObjectRef,
            _property_name: rust_jsc::internal::JSStringRef,
            _exception: *mut rust_jsc::internal::JSValueRef,
        ) -> rust_jsc::internal::JSValueRef {
            // SAFETY: JavaScriptCore invokes static value callbacks with a live
            // context and object for the duration of the callback.
            let ctx = unsafe { JSContext::borrowed(ctx) };
            let object = JSObject::from_ref(object, ctx.inner);
            let count = object
                .get_private_data::<i32>()
                .map(|count| *count as f64)
                .unwrap_or_default();

            JSValue::number(&ctx, count).inner
        }

        unsafe extern "C" fn set_count(
            ctx: rust_jsc::internal::JSContextRef,
            object: rust_jsc::internal::JSObjectRef,
            _property_name: rust_jsc::internal::JSStringRef,
            value: rust_jsc::internal::JSValueRef,
            _exception: *mut rust_jsc::internal::JSValueRef,
        ) -> bool {
            // SAFETY: JavaScriptCore invokes static value callbacks with a live
            // context, object, and value for the duration of the callback.
            let ctx = unsafe { JSContext::borrowed(ctx) };
            let object = JSObject::from_ref(object, ctx.inner);
            let value = JSValue::new(value, ctx.inner);
            let Ok(number) = value.as_number() else {
                return false;
            };
            let Some(mut count) = object.get_private_data_mut::<i32>() else {
                return false;
            };

            *count = number as i32;
            true
        }

        let ctx = JSContext::new();
        let class = JSClass::try_builder("AccessorCounter")
            .unwrap()
            .accessor("count", Some(get_count), Some(set_count))
            .unwrap()
            .build::<i32>()
            .unwrap();
        let counter = class.object::<i32>(&ctx, Some(41));

        ctx.global_object()
            .set_property("counter", &counter, Default::default())
            .unwrap();

        assert_eq!(
            ctx.evaluate_script("counter.count", None)
                .unwrap()
                .as_number()
                .unwrap(),
            41.0
        );
        assert_eq!(
            ctx.evaluate_script("counter.count = 7; counter.count", None)
                .unwrap()
                .as_number()
                .unwrap(),
            7.0
        );
        assert_eq!(*counter.get_private_data::<i32>().unwrap(), 7);
    }

    #[test]
    fn test_class_builder_typed_accessor_converts_values_and_errors() {
        struct CountAccessor;

        impl JSClassAccessor for CountAccessor {
            type Value = i32;

            fn get(_ctx: JSContext, object: JSObject) -> JSResult<Self::Value> {
                Ok(*object.get_private_data::<i32>().unwrap())
            }

            fn set(
                _ctx: JSContext,
                object: JSObject,
                value: Self::Value,
            ) -> JSResult<()> {
                *object.get_private_data_mut::<i32>().unwrap() = value;
                Ok(())
            }
        }

        let ctx = JSContext::new();
        let class = JSClass::try_builder("TypedAccessorCounter")
            .unwrap()
            .typed_accessor::<CountAccessor>("count")
            .unwrap()
            .build::<i32>()
            .unwrap();
        let counter = class.object::<i32>(&ctx, Some(3));

        ctx.global_object()
            .set_property("typedCounter", &counter, Default::default())
            .unwrap();

        assert_eq!(
            ctx.evaluate_script("typedCounter.count", None)
                .unwrap()
                .as_number()
                .unwrap(),
            3.0
        );
        assert_eq!(
            ctx.evaluate_script("typedCounter.count = 11; typedCounter.count", None)
                .unwrap()
                .as_number()
                .unwrap(),
            11.0
        );
        assert_eq!(*counter.get_private_data::<i32>().unwrap(), 11);

        let error = ctx
            .evaluate_script("typedCounter.count = 11.5", None)
            .unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "value cannot be represented as i32"
        );
    }

    #[test]
    fn test_class_initialize() {
        #[constructor]
        fn constructor(
            _ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            println!("Constructor");
            let value = JSValue::string(&_ctx, "John");
            this.set_property("name", &value, Default::default())
                .unwrap();
            Ok(this.into())
        }

        #[initialize]
        fn initialize(_ctx: JSContext, _object: JSObject) {
            println!("Initialize");
        }

        #[finalize]
        fn finalize(_data_ptr: PrivateData) {
            println!("Finalize");
        }

        #[has_instance]
        fn has_instance(
            _ctx: JSContext,
            _constructor: JSObject,
            _instance: JSValue,
        ) -> JSResult<bool> {
            println!("Has instance");
            let name = _constructor
                .get_property("name")
                .unwrap()
                .as_string()
                .unwrap();

            println!("Name: {}", name);
            if name == "John" {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("Test")
            .set_version(1)
            .set_attributes(JSClassAttribute::None.into())
            .set_initialize(Some(initialize))
            .set_finalize(Some(finalize))
            .call_as_function(None)
            .call_as_constructor(Some(constructor))
            .has_instance(Some(has_instance))
            .build::<i32>()
            .unwrap();

        class.register(&ctx).unwrap();
        let result = ctx
            .evaluate_script(
                r#"
                let obj = new Test();
                obj instanceof Test;
            "#,
                None,
            )
            .unwrap();

        assert!(result.is_boolean());
        assert!(result.as_boolean());

        let object = ctx.evaluate_script("obj", None).unwrap();
        assert!(object.is_object_of_class(&class).unwrap());

        let object = object.as_object().unwrap();
        // SAFETY: this test owns the object and intentionally exercises the raw
        // private-data replacement API with a matching data type.
        let result = unsafe { object.set_private_data(42) };
        assert!(result.is_success());
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_class_object_private_data_type_safe() {
        let ctx = JSContext::new();
        let class = JSClass::builder("TypeSafeTest").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("hello")));
        let object = object.as_object().unwrap();

        // Correct type returns data
        assert_eq!(object.get_private_data::<String>().unwrap(), "hello");

        // Wrong type returns None
        assert!(object.get_private_data::<i32>().is_none());
        assert!(object.get_private_data::<Vec<u8>>().is_none());
    }

    #[test]
    fn test_class_object_no_data() {
        let ctx = JSContext::new();
        let class = JSClass::builder("NoDataTest").build::<()>().unwrap();

        let object = class.object::<()>(&ctx, None);
        let object = object.as_object().unwrap();

        // No data was set
        assert!(object.get_private_data::<i32>().is_none());
        assert!(object.get_private_data::<String>().is_none());
    }

    #[test]
    fn test_class_object_with_explicit_prototype() {
        let ctx = JSContext::new();
        let class = JSClass::builder("ManualPrototypeTest")
            .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
            .build::<i32>()
            .unwrap();
        let prototype = JSObject::new(&ctx);
        prototype
            .set_property(
                "label",
                &JSValue::string(&ctx, "manual"),
                Default::default(),
            )
            .unwrap();

        let object = class
            .object_with_prototype::<i32>(&ctx, Some(9), &prototype)
            .unwrap();

        ctx.global_object()
            .set_property("manualObject", &object, Default::default())
            .unwrap();
        assert_eq!(
            ctx.evaluate_script("manualObject.label", None)
                .unwrap()
                .as_string()
                .unwrap(),
            "manual"
        );
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 9);

        let other_ctx = JSContext::new();
        let other_prototype = JSObject::new(&other_ctx);
        let error = match class.object_with_prototype::<i32>(&ctx, None, &other_prototype)
        {
            Ok(_) => panic!("expected cross-context prototype error"),
            Err(error) => error,
        };
        assert_eq!(
            error.message().unwrap().to_string(),
            "prototype object belongs to a different JavaScript context"
        );
    }

    #[test]
    fn test_class_set_object_private_data_safe_path() {
        let ctx = JSContext::new();
        let class = JSClass::builder("SetDataTest").build::<i32>().unwrap();
        let object = class.object::<i32>(&ctx, None);
        let object = object.as_object().unwrap();

        assert_eq!(
            class.set_object_private_data(&object, 42).unwrap(),
            PrivateDataSetStatus::Set
        );
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
        assert_eq!(
            class.set_object_private_data(&object, 100).unwrap(),
            PrivateDataSetStatus::AlreadySet
        );
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_class_set_object_private_data_rejects_wrong_class_or_type() {
        let ctx = JSContext::new();
        let class = JSClass::builder("SetDataReject").build::<i32>().unwrap();
        let string_class = JSClass::builder("SetDataRejectString")
            .build::<String>()
            .unwrap();
        let object = class.object::<i32>(&ctx, None);
        let object = object.as_object().unwrap();
        let plain = JSObject::new(&ctx);

        assert_eq!(
            class.set_object_private_data(&plain, 1).unwrap(),
            PrivateDataSetStatus::Unsupported
        );
        assert_eq!(
            string_class
                .set_object_private_data(&object, String::from("wrong"))
                .unwrap(),
            PrivateDataSetStatus::Unsupported
        );
        assert!(class
            .set_object_private_data(&object, String::from("wrong"))
            .is_err());
        assert!(object.get_private_data::<i32>().is_none());
    }

    #[test]
    fn test_class_object_take_private_data() {
        let ctx = JSContext::new();
        let class = JSClass::builder("TakeDataTest").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("take me")));
        let object = object.as_object().unwrap();

        // Take ownership
        let taken = object.take_private_data::<String>().unwrap();
        assert_eq!(taken, "take me");

        // Data is gone
        assert!(object.get_private_data::<String>().is_none());
    }

    #[test]
    fn test_class_object_take_wrong_type_preserves_data() {
        let ctx = JSContext::new();
        let class = JSClass::builder("TakeWrongTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(42));
        let object = object.as_object().unwrap();

        // Take with wrong type — should return None and preserve data
        assert!(object.take_private_data::<String>().is_none());

        // Data still accessible with correct type
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_safe_private_data_access_ignores_foreign_private_pointer() {
        unsafe extern "C" fn install_foreign_private(
            _ctx: JSContextRef,
            object: JSObjectRef,
        ) {
            // SAFETY: this test intentionally installs a non-rust-jsc private
            // pointer to prove safe accessors do not interpret it as a
            // PrivateDataHeader.
            unsafe {
                JSObjectSetPrivate(object, std::ptr::dangling_mut::<c_void>());
            }
        }

        let ctx = JSContext::new();
        let class = JSClass::try_builder("ForeignPrivate")
            .unwrap()
            .set_initialize(Some(install_foreign_private))
            .build::<()>()
            .unwrap();
        let object = class.object::<()>(&ctx, None);

        assert!(object.get_private_data::<i32>().is_none());
        assert!(object.get_private_data_mut::<i32>().is_none());
        assert!(object.take_private_data::<i32>().is_none());
        assert_eq!(
            object.drop_private_data::<i32>(),
            crate::PrivateDataDropStatus::TypeMismatch
        );
    }

    #[test]
    fn test_class_object_mut_data() {
        let ctx = JSContext::new();
        let class = JSClass::builder("MutDataTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(10));
        let object = object.as_object().unwrap();

        {
            let mut data = object.get_private_data_mut::<i32>().unwrap();
            *data = 99;
        }

        assert_eq!(*object.get_private_data::<i32>().unwrap(), 99);
    }

    #[test]
    fn test_class_object_multiple_reads() {
        let ctx = JSContext::new();
        let class = JSClass::builder("MultiReadTest").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("persistent")));
        let object = object.as_object().unwrap();

        // Multiple immutable reads are fine
        assert_eq!(object.get_private_data::<String>().unwrap(), "persistent");
        assert_eq!(object.get_private_data::<String>().unwrap(), "persistent");
        assert_eq!(object.get_private_data::<String>().unwrap(), "persistent");
    }

    #[test]
    #[should_panic(expected = "Data type does not match class type")]
    fn test_class_object_type_mismatch_panics() {
        let ctx = JSContext::new();
        let class = JSClass::builder("MismatchTest").build::<i32>().unwrap();

        // Attempting to create an object with a different type should panic
        let _object = class.object::<String>(&ctx, Some(String::from("wrong")));
    }

    #[test]
    fn test_class_object_struct_data() {
        #[derive(Debug, PartialEq)]
        struct Config {
            width: u32,
            height: u32,
            title: String,
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("ConfigClass").build::<Config>().unwrap();

        let object = class.object::<Config>(
            &ctx,
            Some(Config {
                width: 800,
                height: 600,
                title: "Window".to_string(),
            }),
        );
        let object = object.as_object().unwrap();

        let config = object.get_private_data::<Config>().unwrap();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.title, "Window");
    }

    #[test]
    fn test_class_refcell_safe_mutation() {
        use std::cell::RefCell;

        let ctx = JSContext::new();
        let class = JSClass::builder("RefCellTest")
            .build::<RefCell<Vec<String>>>()
            .unwrap();

        let object = class.object::<RefCell<Vec<String>>>(
            &ctx,
            Some(RefCell::new(vec!["first".to_string()])),
        );
        let object = object.as_object().unwrap();

        // Safe mutation via RefCell — no unsafe needed
        let cell = object.get_private_data::<RefCell<Vec<String>>>().unwrap();
        cell.borrow_mut().push("second".to_string());

        let cell = object.get_private_data::<RefCell<Vec<String>>>().unwrap();
        assert_eq!(
            &*cell.borrow(),
            &["first".to_string(), "second".to_string()]
        );
    }

    #[test]
    fn test_class_builder_name_lifetime_repeated_creation() {
        let ctx = JSContext::new();

        for index in 0..128 {
            let name = format!("LifetimeClass{index}");
            let class = JSClass::builder(&name).build::<()>().unwrap();
            let object = class.object::<()>(&ctx, None);
            ctx.global_object()
                .set_property("__rust_jsc_lifetime_probe", &object, Default::default())
                .unwrap();

            let tag = ctx
                .evaluate_script(
                    "Object.prototype.toString.call(__rust_jsc_lifetime_probe)",
                    None,
                )
                .unwrap()
                .as_string()
                .unwrap()
                .to_string();

            assert_eq!(tag, format!("[object {name}]"));
        }
    }

    #[test]
    fn test_class_registration_and_inheritance() {
        #[constructor]
        fn constructor(
            ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            this.set_property(
                "created",
                &JSValue::boolean(&ctx, true),
                Default::default(),
            )?;
            Ok(this.into())
        }

        let ctx = JSContext::new();
        let parent = JSClass::builder("PhaseOneParent")
            .call_as_constructor(Some(constructor))
            .build::<()>()
            .unwrap();
        let child = JSClass::builder("PhaseOneChild")
            .parent_class(&parent)
            .call_as_constructor(Some(constructor))
            .build::<()>()
            .unwrap();

        parent.register(&ctx).unwrap();
        child.register(&ctx).unwrap();

        let value = ctx
            .evaluate_script("const child = new PhaseOneChild(); child", None)
            .unwrap();

        assert!(value.is_object_of_class(&child).unwrap());
        assert!(value.is_object_of_class(&parent).unwrap());
        assert!(value
            .as_object()
            .unwrap()
            .get_property("created")
            .unwrap()
            .as_boolean());
        assert!(ctx
            .evaluate_script("child.created", None)
            .unwrap()
            .as_boolean());
    }

    #[test]
    fn test_class_default_finalize_callback_drops_private_data() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct FinalizeProbe;

        impl Drop for FinalizeProbe {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        DROP_COUNT.store(0, Ordering::SeqCst);

        let ctx = JSContext::new();
        let class = JSClass::builder("FinalizeProbe")
            .build::<FinalizeProbe>()
            .unwrap();
        let object = class.object::<FinalizeProbe>(&ctx, Some(FinalizeProbe));

        // SAFETY: this test invokes the class finalizer directly to verify drop
        // behavior, then clears JavaScriptCore's private slot so object cleanup
        // cannot drop the same test allocation twice.
        unsafe {
            JSClassBuilder::finalize_callback::<FinalizeProbe>(object.inner);
            rust_jsc::internal::JSObjectSetPrivate(object.inner, std::ptr::null_mut());
        }

        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
    }
}
