use std::ffi::CString;

use rust_jsc_sys::{
    kJSClassDefinitionEmpty, JSClassCreate, JSClassDefinition, JSClassRelease,
    JSClassRetain, JSObjectCallAsConstructorCallback, JSObjectCallAsFunctionCallback,
    JSObjectConvertToTypeCallback, JSObjectDeletePropertyCallback,
    JSObjectFinalizeCallback, JSObjectGetPropertyCallback,
    JSObjectGetPropertyNamesCallback, JSObjectHasInstanceCallback,
    JSObjectHasPropertyCallback, JSObjectInitializeCallback, JSObjectMake,
    JSObjectSetPropertyCallback,
};

use crate::{JSClass, JSContext, JSObject, JSResult};

#[derive(Debug)]
pub enum ClassError {
    CreateFailed,
    RetainFailed,
}

pub struct JSClassBuilder {
    definition: JSClassDefinition,
    name: String,
}

impl JSClassBuilder {
    pub fn new(name: &str) -> Self {
        let mut definition = unsafe { kJSClassDefinitionEmpty };

        let class_name = CString::new(name).unwrap();
        definition.className = class_name.as_ptr();
        Self {
            definition,
            name: name.to_string(),
        }
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

    /// TODO: implement static values
    /// TODO: implement static functions

    pub fn set_initialize(mut self, initialize: JSObjectInitializeCallback) -> Self {
        self.definition.initialize = initialize;
        self
    }

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

    pub fn build(self) -> Result<JSClass, ClassError> {
        let class = unsafe { JSClassCreate(&self.definition) };
        if class.is_null() {
            return Err(ClassError::CreateFailed);
        }

        let class = unsafe { JSClassRetain(class) };
        if class.is_null() {
            return Err(ClassError::RetainFailed);
        }

        Ok(JSClass {
            inner: class,
            name: self.name,
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
    /// let ctx = JSContext::default();
    /// let class = JSClass::builder("Test")
    ///    .set_version(1)
    ///     .build()
    ///    .unwrap();
    ///
    /// let object = class.object::<i32>(&ctx, Some(Box::new(42)));
    /// ```
    ///
    /// # Returns
    /// A new object of the class.
    pub fn object<T>(&self, ctx: &JSContext, data: Option<Box<T>>) -> JSObject {
        let data_ptr = if let Some(data) = data {
            Box::into_raw(data) as *mut std::ffi::c_void
        } else {
            std::ptr::null_mut()
        };

        let inner = unsafe { JSObjectMake(ctx.inner, self.inner, data_ptr) };
        JSObject::from_ref(inner, ctx.inner)
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
    /// let ctx = JSContext::default();
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
    ///     .build()
    ///     .unwrap();
    ///
    /// class.register(&ctx).unwrap();
    /// ```
    ///
    /// # Errors
    /// If an error occurs while registering the class.
    pub fn register(&self, ctx: &JSContext) -> JSResult<()> {
        ctx.global_object().set_property(
            &self.name().into(),
            &self.object::<()>(ctx, None),
            Default::default(),
        )
    }
}

impl Drop for JSClass {
    fn drop(&mut self) {
        unsafe { JSClassRelease(self.inner) };
    }
}

#[cfg(test)]
mod tests {
    use crate as rust_jsc;
    use rust_jsc_macros::constructor;

    use crate::{JSClass, JSClassAttribute, JSContext, JSObject, JSResult, JSValue};

    #[test]
    fn test_class_builder() {
        #[constructor]
        fn constructor(
            _ctx: JSContext,
            this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let value = JSValue::string(&_ctx, "John");
            this.set_property(&"name".into(), &value, Default::default())
                .unwrap();
            Ok(this.into())
        }

        let ctx = JSContext::default();
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
            .build()
            .unwrap();

        let object = class.object::<i32>(&ctx, Some(Box::new(42)));

        ctx.global_object()
            .set_property(&"Test".into(), &object, Default::default())
            .unwrap();
        let result_object = ctx
            .evaluate_script("const obj = new Test(); obj", None)
            .unwrap();

        assert!(result_object.is_object_of_class(&class).unwrap());
        assert!(object.is_object());
        let object = object.as_object().unwrap();
        assert!(object.has_property(&"name".into()));
        assert_eq!(
            object.get_property(&"name".into()).unwrap(),
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
            this.set_property(&"name".into(), &value, Default::default())
                .unwrap();
            Ok(this.into())
        }

        let ctx = JSContext::default();
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
            .build()
            .unwrap();

        class.register(&ctx).unwrap();
        let result_object = ctx
            .evaluate_script("const obj = new Test(); obj", None)
            .unwrap();

        assert!(result_object.is_object_of_class(&class).unwrap());
    }

    #[test]
    fn test_class_without_constructor() {
        let ctx = JSContext::default();
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
            .build()
            .unwrap();

        class.register(&ctx).unwrap();
        let result = ctx.evaluate_script("const obj = new Test(); obj", None);

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.name().unwrap(), "TypeError");
    }
}
