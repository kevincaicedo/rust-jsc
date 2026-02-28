use std::ops::Deref;

use rust_jsc_sys::{
    JSObjectCallAsConstructorCallback, JSObjectCallAsFunctionCallback,
    JSObjectMakeConstructor, JSObjectMakeFunctionWithCallback,
};

use crate::{JSClass, JSContext, JSFunction, JSObject, JSResult, JSString, JSValue};

impl JSFunction {
    pub(crate) fn new(object: JSObject) -> Self {
        Self { object }
    }

    /// Calls the function with the specified `this` object and arguments.
    /// This is equivalent to `function.call(this, ...arguments)` in JavaScript.
    /// If `this` is `None`, the global object will be used as `this`.
    /// If `arguments` is empty, no arguments will be passed to the function.
    ///
    /// # Arguments
    /// - `this`: The `this` object to use when calling the function.
    /// - `arguments`: The arguments to pass to the function.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::{JSContext, JSFunction, JSObject, JSValue};
    ///
    ///
    /// #[callback]
    /// fn log_error(
    ///     ctx: JSContext,
    ///     _function: JSObject,
    ///     _this: JSObject,
    ///     arguments: &[JSValue],
    /// ) -> JSResult<JSValue> {
    ///     let message = arguments.get(0).unwrap().as_string().unwrap();
    ///     println!("ERROR: {}", message);

    ///     let error = JSError::new_error(&ctx, arguments).unwrap();
    ///     Err(error)
    /// }
    /// let ctx = JSContext::new();
    /// let function = JSFunction::callback(&ctx, Some("log"), Some(log_error));
    /// let result = function.call(None, &[JSValue::string(&ctx, "Hello, World!")]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while calling the function.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The result of calling the function.
    pub fn call(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.object.call(this, arguments)
    }

    /// Calls the function as a constructor with the specified arguments.
    /// This is equivalent to `new function(...arguments)` in JavaScript.
    /// If `arguments` is empty, no arguments will be passed to the constructor.
    ///
    /// # Arguments
    /// - `arguments`: The arguments to pass to the constructor.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::{JSContext, JSFunction, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let function = JSFunction::callback(&ctx, Some("log"), Some(log_error));
    /// let result = function.call_constructor(&[JSValue::string(&ctx, "Hello, World!")]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Errors
    /// If an exception is thrown while calling the constructor.
    /// A `JSError` will be returned.
    ///
    /// # Returns
    /// The result of calling the constructor.
    pub fn call_constructor(&self, arguments: &[JSValue]) -> JSResult<JSObject> {
        self.object.call_as_constructor(arguments)
    }

    /// Returns `true` if the function is a constructor.
    ///
    /// # Returns
    /// `true` if the function is a constructor, otherwise `false`.
    pub fn is_contructor(&self) -> bool {
        self.object.is_contructor()
    }

    /// Creates a new function with the specified name and callback.
    ///
    /// # Arguments
    /// - `name`: The name of the function.
    /// - `callback`: The callback to call when the function is called.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::{JSContext, JSFunction, JSObject, JSValue};
    ///
    /// #[callback]
    /// fn log_error(
    ///     ctx: JSContext,
    ///     _function: JSObject,
    ///     _this: JSObject,
    ///     arguments: &[JSValue],
    /// ) -> JSResult<JSValue> {
    ///      let message = arguments.get(0).unwrap().as_string().unwrap();
    ///      println!("ERROR: {}", message);
    ///      let error = JSError::new_error(&ctx, arguments).unwrap();
    ///      Err(error)
    ///  }
    /// let ctx = JSContext::new();
    /// let function = JSFunction::callback(&ctx, Some("log"), Some(log_error));
    /// let result = function.call(None, &[JSValue::string(&ctx, "Hello, World!")]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Returns
    /// A new function with the specified name and callback.
    pub fn callback<T>(
        ctx: &JSContext,
        name: Option<T>,
        callback: JSObjectCallAsFunctionCallback,
    ) -> Self
    where
        T: Into<JSString>,
    {
        let result = unsafe {
            JSObjectMakeFunctionWithCallback(
                ctx.inner,
                name.map(|name| name.into().inner)
                    .unwrap_or(std::ptr::null_mut()),
                callback,
            )
        };

        let object = JSObject::from_ref(result, ctx.inner);
        Self::new(object)
    }

    /// Creates a new function with the specified name and callback.
    ///
    /// # Arguments
    /// - `name`: The name of the function.
    /// - `callback`: The callback to call when the function is called.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::{JSContext, JSFunction, JSObject, JSValue};
    ///
    /// #[callback]
    /// fn person(
    ///    ctx: JSContext,
    ///   _constructor: JSObject,
    ///   _this: JSObject,
    ///  arguments: &[JSValue],
    /// ) -> JSResult<JSValue> {
    ///     _constructor.set_property(&"name".into(), &arguments.get(0).unwrap(), Default::default());
    ///     Ok(_constructor)
    /// }
    /// let ctx = JSContext::new();
    /// let function = JSFunction::contructor(&ctx, Some("log"), Some(log_error));
    /// let result = function.call(None, &[JSValue::string(&ctx, "Hello, World!")]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Returns
    /// A new function with the specified name and callback.
    pub fn contructor(
        ctx: &JSContext,
        js_class: &JSClass,
        callback: JSObjectCallAsConstructorCallback,
    ) -> Self {
        let result =
            unsafe { JSObjectMakeConstructor(ctx.inner, js_class.inner, callback) };

        let object = JSObject::from_ref(result, ctx.inner);
        Self::new(object)
    }
}

impl Deref for JSFunction {
    type Target = JSValue;

    fn deref(&self) -> &JSValue {
        &self.object.value
    }
}

impl From<JSFunction> for JSObject {
    fn from(function: JSFunction) -> Self {
        function.object
    }
}

impl From<JSFunction> for JSValue {
    fn from(function: JSFunction) -> Self {
        function.object.into()
    }
}

impl From<JSObject> for JSFunction {
    fn from(object: JSObject) -> Self {
        Self::new(object)
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as rust_jsc, JSError, JSString};
    use rust_jsc_macros::{callback, constructor};
    use rust_jsc_sys::{JSContextRef, JSObjectRef, JSValueRef};

    use crate::{
        JSClass, JSContext, JSFunction, JSObject, JSResult, JSValue,
        PropertyDescriptorBuilder,
    };

    #[test]
    fn test_callback() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            __: JSObject,
            message: JSValue,
        ) -> JSResult<JSValue> {
            println!("INFO: {}", message.as_string().unwrap());
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let object = JSObject::new(&ctx);
        let attributes = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
        object.set_property("log", &function, attributes).unwrap();

        global_object
            .set_property("console", &object, attributes)
            .unwrap();

        let result = ctx.evaluate_script("console.log('Hello, World!')", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_callback_with_missing_arguments() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            __: JSObject,
            message: JSValue,
        ) -> JSResult<JSValue> {
            println!("INFO: {}", message.as_string().unwrap());
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let function = JSFunction::callback(&ctx, Some("print"), Some(log_info));
        global_object
            .set_property("print", &function, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("print()", None);
        assert!(result.is_err());

        let error = result.unwrap_err();
        println!("Error: {:?}", error.message().unwrap());
        assert_eq!(error.name().unwrap(), "TypeError");
    }

    #[test]
    fn test_callback_with_invalid_argument_type() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            __: JSObject,
            _private: JSString,
        ) -> JSResult<JSValue> {
            // println!("IS PRIVATE: {}", private);
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let function = JSFunction::callback(&ctx, Some("print"), Some(log_info));
        global_object
            .set_property("print", &function, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("print(Symbol('foo'))", None);
        assert!(result.is_err());

        let error = result.unwrap_err();
        println!("Error: {:?}", error.message().unwrap());
        assert_eq!(error.name().unwrap(), "TypeError");
    }

    #[test]
    fn test_callback_with_invalid_optional_argument_type() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            __: JSObject,
            _private: Option<JSString>,
        ) -> JSResult<JSValue> {
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let function = JSFunction::callback(&ctx, Some("print"), Some(log_info));
        global_object
            .set_property("print", &function, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("print(Symbol('foo'))", None);
        assert!(result.is_err());

        let error = result.unwrap_err();
        println!("Error: {:?}", error.message().unwrap());
        assert_eq!(error.name().unwrap(), "TypeError");
    }

    #[test]
    fn test_callback_with_multiple_arguments() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            __: JSObject,
            item_1: JSString,
            item_2: String,
            item_3: f64,
            item_4: bool,
            item_5: JSObject,
            item_6: JSValue,
            item_7: Option<JSString>,
            item_8: Option<bool>,
        ) -> JSResult<JSValue> {
            println!("INFO: {}", item_1);
            println!("INFO: {}", item_2);
            println!("INFO: {}", item_3);
            println!("INFO: {}", item_4);
            println!("INFO: {}", item_5.as_string().unwrap());
            println!("INFO: {}", item_6.as_string().unwrap());
            println!("INFO: {:?}", item_7);
            println!("INFO: {:?}", item_8);
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let function = JSFunction::callback(&ctx, Some("print"), Some(log_info));
        global_object
            .set_property("print", &function, Default::default())
            .unwrap();

        let result = ctx.evaluate_script(r#"
            print('Hello, World!', 'Hello, World!', 3.14, true, {}, null, 'Hello, World!');
        "#.into(),
        None);

        assert!(!result.is_err());
    }

    #[test]
    fn test_callback_error() {
        #[callback]
        fn log_error(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            message: JSString,
        ) -> JSResult<JSValue> {
            println!("ERROR: {}", message);

            let arguments = vec![JSValue::string(&ctx, "An error occurred")];
            let error = JSError::new(&ctx, arguments.as_slice()).unwrap();
            Err(error)
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let object = JSObject::new(&ctx);
        let attributes = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        let function = JSFunction::callback(&ctx, Some("log"), Some(log_error));
        object.set_property("log", &function, attributes).unwrap();

        global_object
            .set_property("console", &object, attributes)
            .unwrap();

        let result = ctx.evaluate_script("console.log('Hello, World!')", None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().name().unwrap(), "Error");

        ctx.garbage_collect();
        let result = ctx.evaluate_script("console.log('Hello, World 3!')", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_callback_with() {
        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let object = JSObject::new(&ctx);
        let attributes = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();

        struct CallbackState {
            name: String,
            age: u32,
            birth_date: String,
        }

        let state = CallbackState {
            name: "John Doe".into(),
            age: 30,
            birth_date: "1990-01-01".into(),
        };

        ctx.set_shared_data(state);

        unsafe extern "C" fn callback(
            _ctx: JSContextRef,
            _function: JSObjectRef,
            _this_object: JSObjectRef,
            _argument_count: usize,
            _arguments: *const JSValueRef,
            _exception: *mut JSValueRef,
        ) -> JSValueRef {
            let ctx = crate::JSContext::from(_ctx);
            let state = ctx.get_shared_data::<CallbackState>().unwrap();

            println!("Name: {}", state.name);
            println!("Age: {}", state.age);
            println!("Birth Date: {}", state.birth_date);

            assert!(state.name == "John Doe");
            assert!(state.age == 30);
            assert!(state.birth_date == "1990-01-01");
            std::ptr::null_mut()
        }

        let function = JSFunction::callback::<JSString>(&ctx, None, Some(callback));
        object.set_property("log", &function, attributes).unwrap();

        object.set_property("error", &function, attributes).unwrap();

        global_object
            .set_property("console", &object, attributes)
            .unwrap();

        // function.call(None, &[]).unwrap();
        let result = ctx.evaluate_script("console.error('Hello, World!')", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_constructor() {
        #[constructor]
        fn new_object(
            ctx: JSContext,
            _contructor: JSObject,
            arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let name = arguments.get(0).unwrap().as_string().unwrap();
            let age = arguments.get(1).unwrap().as_number().unwrap();

            let object = JSObject::new(&ctx);
            object
                .set_property("name", &JSValue::string(&ctx, name), Default::default())
                .unwrap();
            object
                .set_property("age", &JSValue::number(&ctx, age), Default::default())
                .unwrap();

            Ok(object.into())
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();

        let attributes = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        let class = JSClass::builder("Person").build::<()>().unwrap();
        let function = JSFunction::contructor(&ctx, &class, Some(new_object));
        global_object
            .set_property("Person", &function.into(), attributes)
            .unwrap();

        let result = ctx
            .evaluate_script("const person = new Person('John Doe', 30); person", None);

        assert!(result.is_ok());

        let person = result.unwrap();
        assert!(person.is_object());
        let person = person.as_object().unwrap();
        assert!(person.has_property("name"));
        assert!(person.has_property("age"));

        let name = person.get_property("name").unwrap();
        assert!(name.is_string());
        assert_eq!(name.as_string().unwrap(), "John Doe");

        let age = person.get_property("age").unwrap();
        assert!(age.is_number());
        assert_eq!(age.as_number().unwrap(), 30.0);
    }
}
