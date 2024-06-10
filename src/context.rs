use rust_jsc_sys::{
    JSAPIModuleLoader, JSCheckScriptSyntax, JSContextGetGlobalContext,
    JSContextGetGlobalObject, JSContextGetGroup, JSContextGetSharedData,
    JSContextGroupCreate, JSContextGroupRef, JSContextGroupRelease, JSContextRef,
    JSContextSetSharedData, JSEvaluateScript, JSGarbageCollect,
    JSGetMemoryUsageStatistics, JSGlobalContextCopyName, JSGlobalContextCreate,
    JSGlobalContextCreateInGroup, JSGlobalContextIsInspectable, JSGlobalContextRef,
    JSGlobalContextRelease, JSGlobalContextRetain, JSGlobalContextSetInspectable,
    JSGlobalContextSetName, JSGlobalContextSetUnhandledRejectionCallback,
    JSLinkAndEvaluateModule, JSLoadAndEvaluateModule, JSLoadAndEvaluateModuleFromSource,
    JSLoadModule, JSLoadModuleFromSource, JSSetAPIModuleLoader, JSSetSyntheticModuleKeys,
    JSStringRef, JSValueRef,
};

use crate::{
    JSClass, JSContext, JSContextGroup, JSObject, JSResult, JSString, JSStringRetain,
    JSValue,
};

impl JSContextGroup {
    pub fn new_context(&self) -> JSContext {
        let ctx = unsafe {
            JSGlobalContextCreateInGroup(self.context_group, std::ptr::null_mut())
        };
        JSContext::from(ctx)
    }

    pub fn new_context_with_class(&self, class: &JSClass) -> JSContext {
        let ctx =
            unsafe { JSGlobalContextCreateInGroup(self.context_group, class.inner) };
        JSContext::from(ctx)
    }

    /// Creates a new `JSContextGroup` object.
    pub fn new() -> Self {
        let context_group = unsafe { JSContextGroupCreate() };
        Self { context_group }
    }
}

impl From<JSContextGroupRef> for JSContextGroup {
    fn from(group: JSContextGroupRef) -> Self {
        Self {
            context_group: group,
        }
    }
}

impl Drop for JSContextGroup {
    fn drop(&mut self) {
        unsafe {
            JSContextGroupRelease(self.context_group);
        }
    }
}

impl std::fmt::Debug for JSContextGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSContextGroup").finish()
    }
}

impl JSContext {
    /// Creates a new `JSContext` object.
    ///
    /// Gets a new global context of a JavaScript execution context.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ```
    pub fn new() -> Self {
        let ctx = unsafe { JSGlobalContextCreate(std::ptr::null_mut()) };
        Self { inner: ctx }
    }

    pub fn new_with_class(class: &JSClass) -> Self {
        let ctx = unsafe { JSGlobalContextCreate(class.inner) };
        Self { inner: ctx }
    }

    /// Garbage collects the JavaScript execution context.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.garbage_collect();
    /// ```
    pub fn garbage_collect(&self) {
        unsafe { JSGarbageCollect(self.inner) }
    }

    /// Gets the memory usage statistics of a JavaScript execution context.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let memory_usage_statistics = ctx.get_memory_usage();
    /// let heap_size = memory_usage_statistics.get_property("heapSize").unwrap().as_number().unwrap();
    /// let heap_capacity = memory_usage_statistics.get_property("heapCapacity").unwrap().as_number().unwrap();
    /// let extra_memory_size = memory_usage_statistics.get_property("extraMemorySize").unwrap().as_number().unwrap();
    /// let object_count = memory_usage_statistics.get_property("objectCount").unwrap().as_number().unwrap();
    /// let protected_object_count = memory_usage_statistics.get_property("protectedObjectCount").unwrap().as_number().unwrap();
    /// let global_object_count = memory_usage_statistics.get_property("globalObjectCount").unwrap().as_number().unwrap();
    /// let protected_global_object_count = memory_usage_statistics.get_property("protectedGlobalObjectCount").unwrap().as_number().unwrap();
    /// let object_type_counts = memory_usage_statistics.get_property("objectTypeCounts").unwrap().as_object().unwrap();
    ///
    /// println!("Heap size: {}", heap_size);
    /// println!("Heap capacity: {}", heap_capacity);
    /// println!("Extra memory size: {}", extra_memory_size);
    /// println!("Object count: {}", object_count);
    /// println!("Protected object count: {}", protected_object_count);
    /// println!("Global object count: {}", global_object_count);
    /// println!("Protected global object count: {}", protected_global_object_count);
    /// ```
    ///
    /// # Returns
    ///
    /// Returns a `JSObject` object.
    /// The object contains the following properties:
    ///     heapSize - The size of the heap.
    ///     heapCapacity - The total size of the heap.
    ///     extraMemorySize - The size of the extra memory.
    ///     objectCount - The number of objects.
    ///     protectedObjectCount - The number of protected objects.
    ///     globalObjectCount - The number of global objects.
    ///     protectedGlobalObjectCount - The number of protected global objects.
    ///     objectTypeCounts - An object that contains the count of each object type.
    pub fn get_memory_usage(&self) -> JSObject {
        let result = unsafe { JSGetMemoryUsageStatistics(self.inner) };
        JSObject::from_ref(result, self.inner)
    }

    /// Sets a callback function that is called when a promise is rejected and no handler is provided.
    /// The callback is called with the rejected promise and the reason for the rejection.
    ///
    /// # Arguments
    /// - `function`: A JavaScript function.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::{JSContext, JSObject};
    ///
    /// let ctx = JSContext::new();
    /// let script = "function handleRejection(reason) { console.log('Unhandled rejection:', reason); }; handleRejection";
    /// let function = ctx.evaluate_script(script, None).unwrap();
    /// assert!(function.is_object());
    /// assert!(function.as_object().unwrap().is_function());
    /// let result = ctx.set_unhandled_rejection_callback(function.as_object().unwrap());
    /// ```
    ///
    pub fn set_unhandled_rejection_callback(&self, function: JSObject) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        unsafe {
            JSGlobalContextSetUnhandledRejectionCallback(
                self.inner,
                function.inner,
                &mut exception,
            );
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(())
    }

    /// Checks the syntax of a JavaScript script.
    ///
    /// # Arguments
    /// - `script`: A JavaScript script.
    /// - `starting_line_number`: The line number to start parsing the script.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.check_syntax("console.log('Hello, world!');", 0);
    /// assert!(result.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a `JSError` if the script has a syntax error.
    /// the error type is a SyntaxError.
    pub fn check_syntax(
        &self,
        script: &str,
        starting_line_number: i32,
    ) -> JSResult<bool> {
        let script: JSString = script.into();
        let source_url = std::ptr::null_mut();
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSCheckScriptSyntax(
                self.inner,
                script.inner,
                source_url,
                starting_line_number,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(result)
    }

    pub fn group(&self) -> JSContextGroup {
        let group = unsafe { JSContextGetGroup(self.inner) };
        JSContextGroup::from(group)
    }

    /// Gets the global object of the JavaScript execution context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let global_object = ctx.global_object();
    /// assert_eq!(format!("{:?}", global_object), "JSObject");
    /// ```
    ///
    /// # Returns
    /// Returns a `JSObject` object.
    pub fn global_object(&self) -> JSObject {
        JSObject::from_ref(unsafe { JSContextGetGlobalObject(self.inner) }, self.inner)
    }

    /// Evaluates a JavaScript module.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let filename = "/path/filename.js";
    /// let ctx = JSContext::new();
    /// let result = ctx.evaluate_module(filename);
    /// assert!(result.is_ok());
    /// ```
    ///
    /// It will use a file system module loader to load the module.
    ///
    pub fn evaluate_module(&self, filename: &str) -> JSResult<()> {
        let filename: JSString = filename.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        unsafe { JSLoadAndEvaluateModule(self.inner, filename.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(())
    }

    /// Loads a module.
    /// The module is loaded using the module loader set for the context.
    /// LoadModule:
    ///     - Fetches the module source text.
    ///     - Parses the module source text.
    ///     - Requests dependencies.
    ///
    /// a new entry will be added to the registry with all dependencies satisfied.
    ///
    /// # Arguments
    /// - `key`: The key of the module.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.load_module("test");
    /// assert!(result.is_ok());
    /// ```
    pub fn load_module(&self, key: &str) -> JSResult<()> {
        let module_key: JSString = key.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        unsafe { JSLoadModule(self.inner, module_key.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(())
    }

    /// Links and evaluates a module.
    /// https://262.ecma-international.org/6.0/#sec-moduledeclarationinstantiation
    /// The module is linked and evaluated using the module loader set for the context.
    ///
    /// LinkAndEvaluateModule:
    ///     - Initialize a new module environment.
    ///     - Ensure all the indirect exports are correctly resolved to unique bindings.
    ///     - Instantiate namespace objects and initialize the bindings with them if required.
    ///     - Initialize heap allocated function declarations.
    ///     - Initialize heap allocated variable declarations.
    ///     - link namespace objects to the module environment.
    ///     - set the module environment to the global environment.
    ///     - Evaluate the module.
    ///
    /// # Arguments
    /// - `key`: The key of the module.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.link_and_evaluate_module("test");
    /// assert!(result.is_undefined());
    /// ```
    pub fn link_and_evaluate_module(&self, key: &str) -> JSValue {
        let module_key: JSString = key.into();
        let result = unsafe { JSLinkAndEvaluateModule(self.inner, module_key.inner) };

        JSValue::new(result, self.inner)
    }

    /// Loads a module from source.
    /// The module is loaded using the module loader set for the context.
    ///
    /// # Arguments
    /// - `source`: The source of the module.
    /// - `source_url`: The URL of the source.
    /// - `starting_line_number`: The line number to start parsing the source.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.load_module_from_source("console.log('Hello, World!')", "test.js", 0);
    /// assert!(result.is_ok());
    /// ```
    #[allow(dead_code)]
    fn load_module_from_source(
        &self,
        source: &str,
        source_url: &str,
        starting_line_number: i32,
    ) -> JSResult<()> {
        let source: JSString = source.into();
        let source_url: JSString = source_url.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        unsafe {
            JSLoadModuleFromSource(
                self.inner,
                source.inner,
                source_url.inner,
                starting_line_number,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(())
    }

    /// Evaluates a module from source.
    /// The module is evaluated using the module loader set for the context.
    ///
    /// # Arguments
    /// - `source`: The source of the module.
    /// - `source_url`: The URL of the source.
    /// - `starting_line_number`: The line number to start parsing the source.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.evaluate_module_from_source("console.log('Hello, World!')", "test.js", None);
    /// assert!(result.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a `JSError` if the module has a syntax error.
    pub fn evaluate_module_from_source(
        &self,
        source: &str,
        source_url: &str,
        starting_line_number: Option<i32>
    ) -> JSResult<()> {
        let source: JSString = source.into();
        let source_url: JSString = source_url.into();
        let mut exception: JSValueRef = std::ptr::null_mut();

        unsafe {
            JSLoadAndEvaluateModuleFromSource(
                self.inner,
                source.inner,
                source_url.inner,
                starting_line_number.unwrap_or(1),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(())
    }

    /// Sets the module loader for a context.
    /// The module loader is used to load modules when evaluating a module.
    /// The module loader is called with the module key and the context.
    /// All fn pointers must be provided. so that the module loader can be used.
    ///
    /// # Arguments
    /// - `module_loader`: A module loader.
    pub fn set_module_loader(&self, module_loader: JSAPIModuleLoader) {
        unsafe { JSSetAPIModuleLoader(self.inner, module_loader) };
    }

    /// Sets the keys for all virtual modules.
    /// The keys are used to identify virtual modules when loading modules.
    ///
    /// # Arguments
    /// - `keys`: An array of keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::{JSContext, JSStringRetain};
    ///
    /// let ctx = JSContext::new();
    /// let keys = &[
    ///    JSStringRetain::from("@rust-jsc"),
    /// ];
    /// ctx.set_virtual_module_keys(keys);
    /// ```
    pub fn set_virtual_module_keys(&self, keys: &[JSStringRetain]) {
        let keys: Vec<JSStringRef> = keys.iter().map(|key| key.0).collect();
        unsafe {
            JSSetSyntheticModuleKeys(self.inner, keys.len(), keys.as_ptr());
        };
    }

    /// Evaluates a JavaScript script.
    ///
    /// # Arguments
    /// - `script`: A JavaScript script.
    /// - `starting_line_number`: The line number to start parsing the script.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.evaluate_script("console.log('Hello, world!'); 'kedojs'", Some(0));
    /// assert!(result.is_ok());
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the script has a syntax error.
    pub fn evaluate_script(
        &self,
        script: &str,
        starting_line_number: Option<i32>,
    ) -> JSResult<JSValue> {
        let script: JSString = script.into();
        let this_object = std::ptr::null_mut();
        let source_url = std::ptr::null_mut();
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = unsafe {
            JSEvaluateScript(
                self.inner,
                script.inner,
                this_object,
                source_url,
                starting_line_number.unwrap_or(0),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        Ok(JSValue::new(result, self.inner))
    }

    /// Checks if a context is inspectable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let is_inspectable = ctx.is_inspectable();
    /// assert_eq!(is_inspectable, true);
    /// ```
    ///
    /// # Returns
    /// a boolean value. `true` if the context is inspectable, `false` otherwise.
    pub fn is_inspectable(&self) -> bool {
        unsafe { JSGlobalContextIsInspectable(self.inner) }
    }

    /// Sets whether a context is inspectable.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_inspectable(true);
    /// assert_eq!(ctx.is_inspectable(), true);
    /// ```
    pub fn set_inspectable(&self, inspectable: bool) {
        unsafe { JSGlobalContextSetInspectable(self.inner, inspectable) };
    }

    /// Sets the name exposed when inspecting a context.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_name("KedoJS");
    /// assert_eq!(ctx.get_name().to_string(), "KedoJS");
    /// ```
    pub fn set_name(&self, name: &str) {
        let name: JSString = name.into();
        unsafe { JSGlobalContextSetName(self.inner, name.inner) }
    }

    /// Gets a copy of the name of a context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let name = ctx.get_name();
    /// ```
    ///
    /// # Returns
    ///
    /// Returns a `JSString` object.
    pub fn get_name(&self) -> JSString {
        let name = unsafe { JSGlobalContextCopyName(self.inner) };
        name.into()
    }

    /// Sets shared data for a context.
    ///
    /// # Arguments
    /// - `data`: A shared data.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let data = Box::new(10);
    /// ctx.set_shared_data(data);
    /// ```
    pub fn set_shared_data<T>(&self, data: Box<T>) {
        let data_ptr = Box::into_raw(data);
        unsafe { JSContextSetSharedData(self.inner, data_ptr as _) }
    }

    /// Gets shared data for a context.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let data = Box::new(10);
    /// ctx.set_shared_data(data);
    /// let shared_data = ctx.get_shared_data::<i32>().unwrap();
    /// assert_eq!(*shared_data, 10);
    /// ```
    ///
    /// # Returns
    pub fn get_shared_data<T>(&self) -> Option<Box<T>> {
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };

        if data_ptr.is_null() {
            return None;
        }
        Some(unsafe { Box::from_raw(data_ptr as *mut T) })
    }
}

impl std::fmt::Debug for JSContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSContext").finish()
    }
}

impl Default for JSContext {
    fn default() -> Self {
        JSContext::new()
    }
}

impl From<JSContextRef> for JSContext {
    fn from(context: JSContextRef) -> Self {
        let global_context = unsafe { JSContextGetGlobalContext(context) };
        unsafe {
            JSGlobalContextRetain(global_context);
        }

        Self {
            inner: global_context,
        }
    }
}

impl Drop for JSContext {
    fn drop(&mut self) {
        unsafe {
            JSGlobalContextRelease(self.inner);
        }

        // TODO: Set the pointers to the shared data to null
        // unsafe {
        //     JSContextSetSharedData(self.global_context, std::ptr::null_mut());
        // }
    }
}

impl From<JSGlobalContextRef> for JSContext {
    fn from(ctx: JSGlobalContextRef) -> Self {
        Self { inner: ctx }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{self as rust_jsc};

    use rust_jsc_macros::*;

    #[module_resolve]
    fn module_loader_resolve_virtual(
        _ctx: JSContext,
        _key: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        JSStringRetain::from("@rust-jsc")
    }

    #[module_evaluate]
    fn module_loader_evaluate_virtual(ctx: JSContext, _key: JSValue) -> JSValue {
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "John Doe");
        object.set_property("name", &value, Default::default()).unwrap();

        let default = JSObject::new(&ctx);
        default.set_property("name", &value, Default::default()).unwrap();

        default.set_property("default", &object, Default::default()).unwrap();
        default.into()
    }

    #[module_evaluate]
    fn module_loader_evaluate_no_default_virtual(ctx: JSContext, _key: JSValue) -> JSValue {
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "John Doe");
        object.set_property("name", &value, Default::default()).unwrap();
        object.into()
    }

    #[module_resolve]
    fn module_loader_resolve_non_virtual(
        _ctx: JSContext,
        key: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        let key_value = key.as_string().unwrap();
        // resolve path to file system
        let test_module_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        // module key can start with ./ or ../
        let path = std::path::Path::new(test_module_dir).join(key_value.to_string());
        let module_path = std::fs::canonicalize(path).unwrap();

        JSStringRetain::from(module_path.to_str().unwrap())
    }

    #[module_fetch]
    fn module_loader_fetch(
        _ctx: JSContext,
        _key: JSValue,
        _attributes_value: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringRetain {
        // read file content
        let path_key = _key.as_string().unwrap().to_string();
        println!("Path key: {:?}", path_key);
        // check if the path is a file
        let file_content = match std::fs::read_to_string(&path_key) {
            Ok(content) => content,
            Err(error) => {
                unreachable!("Error reading file: {:?}", error);
            }
        };

        JSStringRetain::from(file_content)
    }

    #[module_import_meta]
    fn module_loader_create_import_meta_properties(
        ctx: JSContext,
        key: JSValue,
        _script_fetcher: JSValue,
    ) -> JSObject {
        let object = JSObject::new(&ctx);
        object
            .set_property("url", &key, Default::default())
            .unwrap();
        object
    }

    #[test]
    fn test_js_context() {
        let ctx = JSContext::new();
        assert_eq!(format!("{:?}", ctx), "JSContext");
    }

    #[test]
    fn test_js_context_name() {
        let ctx = JSContext::new();
        ctx.set_name("KedoJS");
        assert_eq!(ctx.get_name().to_string(), "KedoJS");
    }

    #[test]
    fn test_js_context_group() {
        let group = JSContextGroup::new();
        assert_eq!(format!("{:?}", group), "JSContextGroup");
    }

    #[test]
    fn test_js_context_with_group() {
        let group = JSContextGroup::new();
        let ctx = group.new_context();
        assert_eq!(format!("{:?}", ctx), "JSContext");
    }

    #[test]
    fn test_js_context_garbage_collect() {
        let ctx = JSContext::new();
        ctx.garbage_collect();
    }

    #[test]
    fn test_js_context_check_syntax() {
        let ctx = JSContext::new();
        let script = "console.log('Hello, world!');";
        let result = ctx.check_syntax(script, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_js_context_global_object() {
        let ctx = JSContext::new();
        let global_object = ctx.global_object();
        assert_eq!(format!("{:?}", global_object), "JSObject");
    }

    #[test]
    fn test_js_context_evaluate_module() {
        let filename = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules/test.js");
        let ctx = JSContext::new();
        let result = ctx.evaluate_module(filename);
        assert!(result.is_ok());
    }

    #[test]
    fn test_js_context_evaluate_module_fails() {
        let filename =
            concat!(env!("CARGO_MANIFEST_DIR"), "/non_exist/mock_path/wrong.js");
        let ctx = JSContext::new();
        let result = ctx.evaluate_module(filename);
        assert!(result.is_err());
    }

    #[test]
    fn test_js_context_evaluate_script() {
        let ctx = JSContext::new();
        let script = "console.log('Hello, world!'); 'kedojs'";
        let result = ctx.evaluate_script(script, None);
        assert!(result.is_ok());
    }

    #[test]
    // #[should_panic]
    fn test_js_context_evaluate_module_source() {
        let ctx = JSContext::new();
        let script = "console.log('Hello, world!'); 'kedojs'";
        let result = ctx.evaluate_module_from_source(script, "source.js", None);
        assert!(result.is_ok());
    }

    // #[test]
    // fn test_shared_data() {
    //     let ctx = JSContext::new();
    //     let data = Box::new(10);
    //     ctx.set_shared_data(data.clone());

    //     let mut shared_data = ManuallyDrop::new(ctx.get_shared_data::<i32>().unwrap());
    //     assert_eq!(*shared_data.as_mut(), 10);

    //     let shared_data = ctx.get_shared_data::<i32>().unwrap();
    //     assert_eq!(*shared_data, 10);
    //     // unsafe { ManuallyDrop::drop(&mut shared_data) };
    // }

    // #[test]
    // fn test_shared_data_null() {
    //     let ctx = JSContext::new();
    //     let shared_data = ctx.get_shared_data::<i32>();
    //     assert!(shared_data.is_none());
    // }

    #[test]
    fn test_inspectable() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);
        assert_eq!(ctx.is_inspectable(), true);
    }

    #[test]
    fn test_virtual_module() {
        let ctx = JSContext::new();
        let keys = &[JSStringRetain::from("@rust-jsc")];
        ctx.set_virtual_module_keys(keys);

        let callbacks = JSAPIModuleLoader {
            disableBuiltinFileSystemLoader: false,
            moduleLoaderResolve: Some(module_loader_resolve_virtual),
            moduleLoaderEvaluate: Some(module_loader_evaluate_virtual),
            moduleLoaderFetch: Some(module_loader_fetch),
            moduleLoaderCreateImportMetaProperties: Some(
                module_loader_create_import_meta_properties,
            ),
        };
        ctx.set_module_loader(callbacks);

        // let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        // let test_dir = format!("{}/virtual_module.js", module_test_dir);
        let result = ctx.evaluate_module_from_source(r"
            import lib from '@rust-jsc'; 
            globalThis.lib = lib;
        ", "virtual_module.js", None);
        assert!(result.is_ok());

        let result = ctx.evaluate_script("lib.name", None);

        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value.as_string().unwrap(), "John Doe");

        let result = ctx.evaluate_module_from_source(r"
            import { name } from '@rust-jsc'; 
            globalThis.name = name;
        ", "virtual_module.js", None);
        assert!(result.is_ok());

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");

    }

    #[test]
    fn test_virtual_module_no_default() {
        let ctx = JSContext::new();
        let keys = &[JSStringRetain::from("@rust-jsc")];
        ctx.set_virtual_module_keys(keys);

        let callbacks = JSAPIModuleLoader {
            disableBuiltinFileSystemLoader: false,
            moduleLoaderResolve: Some(module_loader_resolve_virtual),
            moduleLoaderEvaluate: Some(module_loader_evaluate_no_default_virtual),
            moduleLoaderFetch: Some(module_loader_fetch),
            moduleLoaderCreateImportMetaProperties: Some(
                module_loader_create_import_meta_properties,
            ),
        };
        ctx.set_module_loader(callbacks);

        let result = ctx.evaluate_module_from_source(r"
            import { name } from '@rust-jsc'; 
            globalThis.name = name;
        ", "virtual_module.js", None);
        assert!(result.is_ok());

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");

        let result = ctx.evaluate_module_from_source(r"
            import lib from '@rust-jsc'; 
            globalThis.lib = lib;
        ", "virtual_module.js", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_virtual_module() {
        let ctx = JSContext::new();

        let callbacks = JSAPIModuleLoader {
            disableBuiltinFileSystemLoader: true,
            moduleLoaderResolve: Some(module_loader_resolve_non_virtual),
            moduleLoaderEvaluate: Some(module_loader_evaluate_virtual),
            moduleLoaderFetch: Some(module_loader_fetch),
            moduleLoaderCreateImportMetaProperties: Some(
                module_loader_create_import_meta_properties,
            ),
        };
        ctx.set_module_loader(callbacks);

        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        let test_dir = format!("{}/test.js", module_test_dir);
        let result = ctx.evaluate_module(&test_dir);
        assert!(result.is_ok());

        let result = ctx.evaluate_script("message", None);
        assert!(result.is_ok());

        let result_value = result.unwrap();
        assert_eq!(result_value.as_string().unwrap(), "Hello World KEDO");
    }

    #[test]
    fn test_set_unhandled_rejection_callback() {
        let ctx = JSContext::new();
        let script = "function handleRejection(reason) { console.log('Unhandled rejection:', reason); }; handleRejection";
        let function = ctx.evaluate_script(script, None).unwrap();

        assert!(function.is_object());
        assert!(function.as_object().unwrap().is_function());
        let result = ctx.set_unhandled_rejection_callback(function.as_object().unwrap());
        assert!(result.is_ok());
    }

    #[allow(dead_code)]
    fn memory_usage(ctx: &JSContext) {
        let memory_usage_statistics = ctx.get_memory_usage();
        let heap_size = memory_usage_statistics
            .get_property("heapSize")
            .unwrap()
            .as_number()
            .unwrap();
        let heap_capacity = memory_usage_statistics
            .get_property("heapCapacity")
            .unwrap()
            .as_number()
            .unwrap();
        let extra_memory_size = memory_usage_statistics
            .get_property("extraMemorySize")
            .unwrap()
            .as_number()
            .unwrap();
        let object_count = memory_usage_statistics
            .get_property("objectCount")
            .unwrap()
            .as_number()
            .unwrap();
        let protected_object_count = memory_usage_statistics
            .get_property("protectedObjectCount")
            .unwrap()
            .as_number()
            .unwrap();
        let global_object_count = memory_usage_statistics
            .get_property("globalObjectCount")
            .unwrap()
            .as_number()
            .unwrap();
        let protected_global_object_count = memory_usage_statistics
            .get_property("protectedGlobalObjectCount")
            .unwrap()
            .as_number()
            .unwrap();

        println!("Heap size: {}", heap_size);
        println!("Heap capacity: {}", heap_capacity);
        println!("Extra memory size: {}", extra_memory_size);
        println!("Object count: {}", object_count);
        println!("Protected object count: {}", protected_object_count);
        println!("Global object count: {}", global_object_count);
        println!(
            "Protected global object count: {}",
            protected_global_object_count
        );
    }
}
