use std::any::TypeId;
use std::ffi::CString;

use crate::{
    self as rust_jsc, finalize, JSClass, JSContext, JSObject, JSResult, PrivateData,
    PrivateDataWrapper,
};
use rust_jsc_sys::{
    kJSClassDefinitionEmpty, JSClassCreate, JSClassDefinition, JSClassRelease,
    JSClassRetain, JSObjectCallAsConstructorCallback, JSObjectCallAsFunctionCallback,
    JSObjectConvertToTypeCallback, JSObjectDeletePropertyCallback,
    JSObjectFinalizeCallback, JSObjectGetPropertyCallback,
    JSObjectGetPropertyNamesCallback, JSObjectHasInstanceCallback,
    JSObjectHasPropertyCallback, JSObjectInitializeCallback, JSObjectMake,
    JSObjectSetPropertyCallback,
};

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

    #[finalize]
    fn finalize_callback<T: 'static>(data_ptr: PrivateData) {
        let _ = unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
    }

    pub fn build<T: 'static>(mut self) -> Result<JSClass, ClassError> {
        if self.definition.finalize.is_none() && TypeId::of::<T>() != TypeId::of::<()>() {
            self.definition.finalize = Some(Self::finalize_callback::<T>);
        }

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
            type_id: TypeId::of::<T>(),
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

        let inner = unsafe { JSObjectMake(ctx.inner, self.inner, data_ptr) };
        JSObject::from_ref(inner, ctx.inner)
    }

    fn object_empty(&self, ctx: &JSContext) -> JSObject {
        let inner = unsafe { JSObjectMake(ctx.inner, self.inner, std::ptr::null_mut()) };
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
    ///     .build::<()>()
    ///     .unwrap();
    ///
    /// class.register(&ctx).unwrap();
    /// ```
    ///
    /// # Errors
    /// If an error occurs while registering the class.
    pub fn register(&self, ctx: &JSContext) -> JSResult<()> {
        ctx.global_object().set_property(
            self.name(),
            &self.object_empty(ctx),
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
    use crate::{self as rust_jsc, PrivateData};
    use rust_jsc_macros::{constructor, finalize, has_instance, initialize};

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
            this.set_property("name", &value, Default::default())
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
            .build::<()>()
            .unwrap();

        class.register(&ctx).unwrap();
        let result = ctx.evaluate_script("const obj = new Test(); obj", None);

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.name().unwrap(), "TypeError");
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

        let ctx = JSContext::default();
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
        assert_eq!(result.as_boolean(), true);

        let object = ctx.evaluate_script("obj", None).unwrap();
        assert!(object.is_object_of_class(&class).unwrap());

        let object = object.as_object().unwrap();
        let result = unsafe { object.set_private_data(42) };
        assert!(result);
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_class_object_private_data_type_safe() {
        let ctx = JSContext::default();
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
        let ctx = JSContext::default();
        let class = JSClass::builder("NoDataTest").build::<()>().unwrap();

        let object = class.object::<()>(&ctx, None);
        let object = object.as_object().unwrap();

        // No data was set
        assert!(object.get_private_data::<i32>().is_none());
        assert!(object.get_private_data::<String>().is_none());
    }

    #[test]
    fn test_class_object_take_private_data() {
        let ctx = JSContext::default();
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
        let ctx = JSContext::default();
        let class = JSClass::builder("TakeWrongTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(42));
        let object = object.as_object().unwrap();

        // Take with wrong type — should return None and preserve data
        assert!(object.take_private_data::<String>().is_none());

        // Data still accessible with correct type
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_class_object_mut_data() {
        let ctx = JSContext::default();
        let class = JSClass::builder("MutDataTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(10));
        let object = object.as_object().unwrap();

        // SAFETY: no other references to this private data exist
        let data = unsafe { object.get_private_data_mut::<i32>() }.unwrap();
        *data = 99;

        assert_eq!(*object.get_private_data::<i32>().unwrap(), 99);
    }

    #[test]
    fn test_class_object_multiple_reads() {
        let ctx = JSContext::default();
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
        let ctx = JSContext::default();
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

        let ctx = JSContext::default();
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

        let ctx = JSContext::default();
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
}
