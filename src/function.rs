use std::ops::Deref;

use rust_jsc_sys::{
    JSObjectCallAsConstructorCallback, JSObjectCallAsFunctionCallback,
    JSObjectMakeConstructor, JSObjectMakeFunctionWithCallback,
};

use crate::{
    JSClass, JSContext, JSFunction, JSObject, JSResult, JSString, JSValue, TryFromJSValue,
};

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
    ///
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

    /// Calls the function and converts the return value.
    ///
    /// This uses [`JSFunction::call`] for JavaScriptCore exception behavior,
    /// then converts the result through [`TryFromJSValue`].
    pub fn call_typed<T>(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<T>
    where
        T: TryFromJSValue,
    {
        self.object.call_typed(this, arguments)
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

    /// Calls the function as a constructor and converts the constructed object.
    ///
    /// This uses [`JSFunction::call_constructor`] for JavaScriptCore exception
    /// behavior, then converts the result through [`TryFromJSValue`].
    pub fn call_constructor_typed<T>(&self, arguments: &[JSValue]) -> JSResult<T>
    where
        T: TryFromJSValue,
    {
        self.object.call_as_constructor_typed(arguments)
    }

    /// Returns `true` if the function is a constructor.
    ///
    /// # Returns
    /// `true` if the function is a constructor, otherwise `false`.
    pub fn is_constructor(&self) -> bool {
        self.object.is_constructor()
    }

    /// Returns the function's JavaScript `name` property as a Rust string.
    ///
    /// This uses ordinary JavaScript property access, so accessor failures or
    /// conversion failures are returned as [`JSError`](crate::JSError).
    pub fn name(&self) -> JSResult<String> {
        let value = self.object.get_property("name")?;
        String::try_from_js_value(&value)
    }

    /// Returns the function's JavaScript `displayName` property when present.
    ///
    /// `undefined` and `null` are reported as `None`. Any other value is
    /// converted through JavaScript string conversion.
    pub fn display_name(&self) -> JSResult<Option<String>> {
        let value = self.object.get_property("displayName")?;
        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        String::try_from_js_value(&value).map(Some)
    }

    /// Returns the function source string produced by JavaScript `toString`.
    ///
    /// This preserves observable JavaScript method lookup on the function
    /// object and returns a [`JSError`](crate::JSError) if lookup, call, or
    /// string conversion throws.
    pub fn source(&self) -> JSResult<String> {
        self.object.call_method_typed("toString", &[])
    }

    /// Deprecated misspelled alias for [`JSFunction::is_constructor`].
    #[deprecated(
        since = "1.0.0",
        note = "use is_constructor; is_contructor will be removed after the 1.0 migration window"
    )]
    pub fn is_contructor(&self) -> bool {
        self.is_constructor()
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
        // SAFETY: `ctx.inner` is a live context. The optional name string is
        // converted to a live `JSStringRef` for the duration of the call, and
        // JavaScriptCore stores the C callback pointer without Rust ownership.
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
    /// let function = JSFunction::constructor(&ctx, Some("log"), Some(log_error));
    /// let result = function.call(None, &[JSValue::string(&ctx, "Hello, World!")]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Returns
    /// A new function with the specified name and callback.
    pub fn constructor(
        ctx: &JSContext,
        js_class: &JSClass,
        callback: JSObjectCallAsConstructorCallback,
    ) -> Self {
        let result =
            // SAFETY: `ctx.inner` and `js_class.inner` are live JavaScriptCore
            // handles. JavaScriptCore stores the constructor callback pointer
            // without taking ownership of Rust data.
            unsafe { JSObjectMakeConstructor(ctx.inner, js_class.inner, callback) };

        let object = JSObject::from_ref(result, ctx.inner);
        Self::new(object)
    }

    /// Deprecated misspelled alias for [`JSFunction::constructor`].
    #[deprecated(
        since = "1.0.0",
        note = "use constructor; contructor will be removed after the 1.0 migration window"
    )]
    pub fn contructor(
        ctx: &JSContext,
        js_class: &JSClass,
        callback: JSObjectCallAsConstructorCallback,
    ) -> Self {
        Self::constructor(ctx, js_class, callback)
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
        CallbackContext, JSClass, JSContext, JSFunction, JSObject, JSResult, JSValue,
        PropertyDescriptorBuilder, ThisObject, TryFromJSValue,
    };

    #[test]
    fn test_callback() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            _this: JSObject,
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
            _this: JSObject,
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
            _this: JSObject,
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
            _this: JSObject,
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
    fn test_callback_optional_argument_accepts_undefined_and_null() {
        #[callback]
        fn optional_len(
            ctx: JSContext,
            _: JSObject,
            _this: JSObject,
            value: Option<JSString>,
        ) -> JSResult<JSValue> {
            let len = value.map(|value| value.len()).unwrap_or(0);
            Ok(JSValue::number(&ctx, len as f64))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();
        let function =
            JSFunction::callback(&ctx, Some("optionalLen"), Some(optional_len));
        global_object
            .set_property("optionalLen", &function, Default::default())
            .unwrap();

        let result = ctx
            .evaluate_script(
                "optionalLen(undefined) + optionalLen(null) + optionalLen('abcd')",
                None,
            )
            .unwrap();

        assert_eq!(result.as_number().unwrap(), 4.0);
    }

    #[test]
    fn test_callback_with_rest_arguments() {
        #[callback]
        fn sum(
            ctx: JSContext,
            _: JSObject,
            _this: JSObject,
            first: f64,
            rest: crate::Rest<f64>,
        ) -> JSResult<JSValue> {
            let total = first + rest.iter().sum::<f64>();
            Ok(JSValue::number(&ctx, total))
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();
        let function = JSFunction::callback(&ctx, Some("sum"), Some(sum));
        global_object
            .set_property("sum", &function, Default::default())
            .unwrap();

        let result = ctx.evaluate_script("sum(1, 2, 3, 4)", None).unwrap();

        assert_eq!(result.as_number().unwrap(), 10.0);
    }

    #[test]
    fn test_function_call_typed_converts_return_value() {
        #[callback]
        fn add(
            ctx: JSContext,
            _this: JSObject,
            _function: JSObject,
            left: f64,
            right: f64,
        ) -> JSResult<JSValue> {
            Ok(JSValue::number(&ctx, left + right))
        }

        #[callback]
        fn text(
            ctx: JSContext,
            _this: JSObject,
            _function: JSObject,
        ) -> JSResult<JSValue> {
            Ok(JSValue::string(&ctx, "not-an-integer"))
        }

        let ctx = JSContext::new();
        let add = JSFunction::callback(&ctx, Some("add"), Some(add));
        let args = [JSValue::number(&ctx, 2.0), JSValue::number(&ctx, 3.0)];
        let result: f64 = add.call_typed(None, &args).unwrap();
        assert_eq!(result, 5.0);

        let text = JSFunction::callback(&ctx, Some("text"), Some(text));
        assert!(text.call_typed::<u32>(None, &[]).is_err());
    }

    #[test]
    fn test_function_name_display_name_and_source_helpers() {
        let ctx = JSContext::new();
        let value = ctx
            .evaluate_script("(function scriptedName(a, b) { return a + b; })", None)
            .unwrap();
        let function = JSFunction::try_from_js_value(&value).unwrap();

        assert_eq!(function.name().unwrap(), "scriptedName");
        assert_eq!(function.display_name().unwrap(), None);

        let object: JSObject = function.clone().into();
        object
            .set_property(
                "displayName",
                &JSValue::string(&ctx, "friendlyName"),
                Default::default(),
            )
            .unwrap();
        assert_eq!(
            function.display_name().unwrap().as_deref(),
            Some("friendlyName")
        );

        let source = function.source().unwrap();
        assert!(source.contains("scriptedName"));
        assert!(source.contains("return a + b"));
    }

    #[test]
    fn test_callback_without_abi_prefix_converts_arguments() {
        #[callback]
        fn add(left: f64, right: f64) -> f64 {
            left + right
        }

        let ctx = JSContext::new();
        let add = JSFunction::callback(&ctx, Some("add"), Some(add));
        let args = [JSValue::number(&ctx, 2.0), JSValue::number(&ctx, 3.0)];

        let result: f64 = add.call_typed(None, &args).unwrap();
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_callback_context_and_this_injection() {
        #[callback]
        fn add_to_base(
            ctx: CallbackContext,
            this: ThisObject,
            value: f64,
        ) -> JSResult<JSValue> {
            let base = this.get_property("base")?.as_number()?;
            Ok(JSValue::number(&ctx, base + value))
        }

        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        object
            .set_property("base", &JSValue::number(&ctx, 10.0), Default::default())
            .unwrap();
        let add_to_base =
            JSFunction::callback(&ctx, Some("addToBase"), Some(add_to_base));
        object
            .set_property("addToBase", &add_to_base, Default::default())
            .unwrap();

        let result: f64 = object
            .call_method_typed("addToBase", &[JSValue::number(&ctx, 7.0)])
            .unwrap();
        assert_eq!(result, 17.0);
    }

    #[test]
    fn test_callback_with_multiple_arguments() {
        #[callback]
        #[allow(clippy::too_many_arguments)]
        fn log_info(
            ctx: JSContext,
            _: JSObject,
            _this: JSObject,
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

        let result = ctx.evaluate_script(
            r#"
            print('Hello, World!', 'Hello, World!', 3.14, true, {}, null, 'Hello, World!');
        "#,
            None,
        );

        assert!(result.is_ok());
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
            // SAFETY: JavaScriptCore passes a live borrowed callback context;
            // this wrapper does not retain or release it.
            let ctx = unsafe { crate::JSContext::borrowed(_ctx) };
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
            _constructor: JSObject,
            arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let name = arguments.first().unwrap().as_string().unwrap();
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
        let function = JSFunction::constructor(&ctx, &class, Some(new_object));
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

    #[test]
    fn test_constructor_with_typed_arguments() {
        #[constructor]
        fn new_object(
            ctx: JSContext,
            _constructor: JSObject,
            name: String,
            age: Option<f64>,
            tags: crate::Rest<String>,
        ) -> JSResult<JSValue> {
            let object = JSObject::new(&ctx);
            object
                .set_property("name", &JSValue::string(&ctx, name), Default::default())
                .unwrap();
            object
                .set_property(
                    "age",
                    &JSValue::number(&ctx, age.unwrap_or_default()),
                    Default::default(),
                )
                .unwrap();
            object
                .set_property(
                    "tagCount",
                    &JSValue::number(&ctx, tags.len() as f64),
                    Default::default(),
                )
                .unwrap();

            Ok(object.into())
        }

        let ctx = JSContext::new();
        let global_object = ctx.global_object();
        let class = JSClass::builder("Person").build::<()>().unwrap();
        let function = JSFunction::constructor(&ctx, &class, Some(new_object));
        global_object
            .set_property("Person", &function.into(), Default::default())
            .unwrap();

        let person = ctx
            .evaluate_script("new Person('Ada', 36, 'math', 'runtime')", None)
            .unwrap()
            .as_object()
            .unwrap();

        assert_eq!(
            person
                .get_property("name")
                .unwrap()
                .as_string()
                .unwrap()
                .to_string(),
            "Ada"
        );
        assert_eq!(
            person.get_property("age").unwrap().as_number().unwrap(),
            36.0
        );
        assert_eq!(
            person
                .get_property("tagCount")
                .unwrap()
                .as_number()
                .unwrap(),
            2.0
        );
    }

    #[test]
    fn test_function_call_constructor_typed_converts_constructed_object() {
        #[constructor]
        fn new_object(
            ctx: JSContext,
            _constructor: JSObject,
            label: String,
        ) -> JSResult<JSValue> {
            let object = JSObject::new(&ctx);
            object
                .set_property("label", &JSValue::string(&ctx, label), Default::default())
                .unwrap();
            Ok(object.into())
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("TypedCtor").build::<()>().unwrap();
        let constructor = JSFunction::constructor(&ctx, &class, Some(new_object));
        let args = [JSValue::string(&ctx, "runtime")];

        let object: JSObject = constructor.call_constructor_typed(&args).unwrap();
        assert_eq!(
            object
                .get_property("label")
                .unwrap()
                .as_string()
                .unwrap()
                .to_string(),
            "runtime"
        );
    }

    #[test]
    fn test_constructor_context_injection() {
        #[constructor]
        fn new_object(ctx: CallbackContext, label: String) -> JSResult<JSObject> {
            let object = JSObject::new(&ctx);
            object
                .set_property("label", &JSValue::string(&ctx, label), Default::default())
                .unwrap();
            Ok(object)
        }

        let ctx = JSContext::new();
        let class = JSClass::builder("InjectedCtor").build::<()>().unwrap();
        let constructor = JSFunction::constructor(&ctx, &class, Some(new_object));
        let args = [JSValue::string(&ctx, "runtime")];

        let object: JSObject = constructor.call_constructor_typed(&args).unwrap();
        assert_eq!(
            object
                .get_property("label")
                .unwrap()
                .as_string()
                .unwrap()
                .to_string(),
            "runtime"
        );
    }
}
