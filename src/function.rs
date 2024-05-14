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
    pub fn callback(
        ctx: &JSContext,
        name: Option<impl Into<JSString>>,
        callback: JSObjectCallAsFunctionCallback,
    ) -> Self {
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
    use rust_jsc_macros::{callback, constructor};
    use rust_jsc_sys::{JSContextRef, JSObjectRef, JSValueRef};

    use crate::{
        JSClass, JSContext, JSError, JSFunction, JSObject, JSResult, JSValue,
        PropertyDescriptorBuilder,
    };

    #[test]
    fn test_callback() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let message = arguments.get(0).unwrap().as_string().unwrap();
            println!("INFO: {}", message);

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
        object
            .set_property(&"log".into(), &function.into(), attributes)
            .unwrap();

        global_object
            .set_property(&"console".into(), &object.into(), attributes)
            .unwrap();

        let result = ctx.evaluate_script("console.log('Hello, World!')", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_callback_error() {
        #[callback]
        fn log_error(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let message = arguments.get(0).unwrap().as_string().unwrap();
            println!("ERROR: {}", message);

            let error = JSError::new_error(&ctx, arguments).unwrap();
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
        object
            .set_property(&"log".into(), &function.into(), attributes)
            .unwrap();

        global_object
            .set_property(&"console".into(), &object.into(), attributes)
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

        let state = Box::new(state);
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

            println!("Name: {}", state.as_ref().name);
            println!("Age: {}", state.as_ref().age);
            println!("Birth Date: {}", state.as_ref().birth_date);

            assert!(state.as_ref().name == "John Doe");
            assert!(state.as_ref().age == 30);
            assert!(state.as_ref().birth_date == "1990-01-01");
            std::ptr::null_mut()
        }

        let function = JSFunction::callback(&ctx, Some("log"), Some(callback));
        object
            .set_property(&"log".into(), &function.into(), attributes)
            .unwrap();

        global_object
            .set_property(&"console".into(), &object.into(), attributes)
            .unwrap();

        let result = ctx.evaluate_script("console.log('Hello, World!')", None);
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
                .set_property(
                    &"name".into(),
                    &JSValue::string(&ctx, name),
                    Default::default(),
                )
                .unwrap();
            object
                .set_property(
                    &"age".into(),
                    &JSValue::number(&ctx, age),
                    Default::default(),
                )
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
        let class = JSClass::builder("Person").build().unwrap();
        let function = JSFunction::contructor(&ctx, &class, Some(new_object));
        global_object
            .set_property(&"Person".into(), &function.into(), attributes)
            .unwrap();

        let result = ctx
            .evaluate_script("const person = new Person('John Doe', 30); person", None);

        assert!(result.is_ok());

        let person = result.unwrap();
        assert!(person.is_object());
        let person = person.as_object().unwrap();
        assert!(person.has_property(&"name".into()));
        assert!(person.has_property(&"age".into()));

        let name = person.get_property(&"name".into()).unwrap();
        assert!(name.is_string());
        assert_eq!(name.as_string().unwrap(), "John Doe");

        let age = person.get_property(&"age".into()).unwrap();
        assert!(age.is_number());
        assert_eq!(age.as_number().unwrap(), 30.0);
    }
}
