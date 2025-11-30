use crate::{
    JSClass, JSContext, JSContextGroup, JSObject, JSResult, JSString, JSStringProctected,
    JSValue,
};
use rust_jsc_sys::{
    JSAPIModuleLoader, JSCheckScriptSyntax, JSContextGetGlobalContext,
    JSContextGetGlobalObject, JSContextGetGroup, JSContextGetSharedData,
    JSContextGroupCreate, JSContextGroupRef, JSContextGroupRelease, JSContextRef,
    JSContextSetSharedData, JSEvaluateScript, JSGarbageCollect,
    JSGetMemoryUsageStatistics, JSGlobalContextCopyName, JSGlobalContextCreate,
    JSGlobalContextCreateInGroup, JSGlobalContextIsInspectable, JSGlobalContextRef,
    JSGlobalContextRelease, JSGlobalContextSetInspectable, JSGlobalContextSetName,
    JSGlobalContextSetUncaughtExceptionAtEventLoopCallback,
    JSGlobalContextSetUncaughtExceptionHandler,
    JSGlobalContextSetUnhandledRejectionCallback, JSInspectorCleanup,
    JSInspectorDisconnect, JSInspectorIsConnected, JSInspectorSendMessage,
    JSInspectorSetCallback, JSLinkAndEvaluateModule, JSLoadAndEvaluateModule,
    JSLoadAndEvaluateModuleFromSource, JSLoadModule, JSLoadModuleFromSource,
    JSSetAPIModuleLoader, JSSetSyntheticModuleKeys, JSStringRef,
    JSUncaughtExceptionAtEventLoop, JSUncaughtExceptionHandler, JSValueRef,
};
use std::ffi::CString;

impl JSContextGroup {
    pub fn new_context(&self) -> JSContext {
        let ctx = unsafe {
            JSGlobalContextCreateInGroup(self.context_group, std::ptr::null_mut())
        };
        JSContext::from(ctx)
    }

    pub fn new_context_with_class<T>(&self, class: &JSClass<T>) -> JSContext {
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

    pub fn new_with<T>(class: &JSClass<T>) -> Self {
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

    /// Sets a callback function that is called when an exception is not caught.
    /// The callback is called with the exception value.
    /// The callback is called on the context thread.
    ///
    /// # Arguments
    /// - `handler`: A native function
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// #[uncaught_exception]
    /// fn uncaught_exception_handler(ctx: JSContext, filename: JSString, exception: JSValue) {
    ///    println!("Uncaught exception: {:?}", exception.as_json_string(1));
    /// }
    ///
    /// fn main() {
    ///     let ctx = JSContext::new();
    ///     ctx.set_uncaught_exception_handler(uncaught_exception_handler);
    /// }
    /// ```
    pub fn set_uncaught_exception_handler(&self, handler: JSUncaughtExceptionHandler) {
        unsafe {
            JSGlobalContextSetUncaughtExceptionHandler(self.inner, handler);
        };
    }

    /// Sets a callback function that is called when an exception is not caught at the event loop.
    /// The callback is called with the exception value.
    /// The callback is called on the event loop thread.
    ///
    /// # Arguments
    /// - `callback`: A native function
    ///
    /// # Examples
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// #[uncaught_exception_event_loop]
    /// fn uncaught_exception_event_loop(ctx: JSContext, exception: JSValue) {
    ///   println!("Uncaught exception: {:?}", exception.as_json_string(1));
    /// }
    ///
    /// fn main() {
    ///     let ctx = JSContext::new();
    ///     ctx.set_uncaught_exception_at_event_loop_callback(uncaught_exception_event_loop);
    /// }
    /// ```
    pub fn set_uncaught_exception_at_event_loop_callback(
        &self,
        callback: JSUncaughtExceptionAtEventLoop,
    ) {
        unsafe {
            JSGlobalContextSetUncaughtExceptionAtEventLoopCallback(self.inner, callback);
        };
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
        starting_line_number: Option<i32>,
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
    /// use rust_jsc::{JSContext, JSStringProctected};
    ///
    /// let ctx = JSContext::new();
    /// let keys = &[
    ///    JSStringProctected::from("@rust-jsc"),
    /// ];
    /// ctx.set_virtual_module_keys(keys);
    /// ```
    pub fn set_virtual_module_keys(&self, keys: &[JSStringProctected]) {
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

    /// Sets the callback function for inspector messages.
    /// This creates a new rust frontend.
    ///
    /// # Arguments
    /// * `callback` - The callback function to be set.
    ///
    /// # Example
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    ///
    /// #[inspector_callback]
    /// fn inspector_callback(message: &str) {
    ///    println!("Inspector message macro: {}", message);
    /// }
    ///
    /// ctx.set_inspector_callback(inspector_callback);
    /// ```
    pub fn set_inspector_callback(
        &self,
        callback: unsafe extern "C" fn(message: *const std::os::raw::c_char),
    ) {
        unsafe {
            JSInspectorSetCallback(self.inner, Some(callback));
        }
    }

    /// Sends a message to the inspector.
    ///
    /// # Arguments
    /// * `message` - The message to be sent.
    ///
    /// # Example
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    ///
    /// ctx.inspector_send_message("{ method: \"Runtime.evaluate\", params: { expression: \"1 + 1\" } }");
    /// ```
    pub fn inspector_send_message(&self, message: &str) {
        let class_name = CString::new(message).unwrap();
        unsafe {
            JSInspectorSendMessage(self.inner, class_name.as_ptr());
        }
    }

    /// Disconnects the inspector frontend from this context.
    /// This should be called before dropping a context that has an active inspector connection.
    /// After calling this function, no more inspector callbacks will be received for this context.
    ///
    /// # Example
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// // ... set up inspector callback and use it ...
    /// ctx.inspector_disconnect(); // Clean up before context is dropped
    /// ```
    pub fn inspector_disconnect(&self) {
        unsafe {
            JSInspectorDisconnect(self.inner);
        }
    }

    /// Checks if the inspector is currently connected for this context.
    ///
    /// # Returns
    /// `true` if an inspector frontend is connected, `false` otherwise.
    ///
    /// # Example
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// assert_eq!(ctx.inspector_is_connected(), false);
    /// ```
    pub fn inspector_is_connected(&self) -> bool {
        unsafe { JSInspectorIsConnected(self.inner) }
    }

    /// Releases the context.
    ///
    /// # Example
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    ///
    /// ctx.release();
    /// ```
    pub fn release(&self) {
        unsafe {
            JSGlobalContextRelease(self.inner);
        }
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
        // unsafe {
        //     JSGlobalContextRetain(global_context);
        // }

        Self {
            inner: global_context,
        }
    }
}

impl Drop for JSContext {
    fn drop(&mut self) {
        // Disconnect inspector if connected to prevent use-after-free
        // unsafe {
        //     if JSInspectorIsConnected(self.inner) {
        //         JSInspectorDisconnect(self.inner);
        //     }
        // }

        // TODO: Set pointer to null
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
    ) -> JSStringProctected {
        JSStringProctected::from("@rust-jsc")
    }

    #[module_evaluate]
    fn module_loader_evaluate_virtual(ctx: JSContext, _key: JSValue) -> JSValue {
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "John Doe");
        object
            .set_property("name", &value, Default::default())
            .unwrap();

        let default = JSObject::new(&ctx);
        default
            .set_property("name", &value, Default::default())
            .unwrap();

        default
            .set_property("default", &object, Default::default())
            .unwrap();
        default.into()
    }

    #[module_evaluate]
    fn module_loader_evaluate_no_default_virtual(
        ctx: JSContext,
        _key: JSValue,
    ) -> JSValue {
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "John Doe");
        object
            .set_property("name", &value, Default::default())
            .unwrap();
        object.into()
    }

    #[uncaught_exception]
    fn uncaught_exception_handler(
        _ctx: JSContext,
        _filename: JSString,
        exception: JSValue,
    ) {
        println!("Uncaught exception: {:?}", exception.as_json_string(1));
    }

    #[uncaught_exception_event_loop]
    fn uncaught_exception_event_loop(_ctx: JSContext, exception: JSValue) {
        println!("Uncaught exception: {:?}", exception.as_json_string(1));
    }

    #[module_resolve]
    fn module_loader_resolve_non_virtual(
        _ctx: JSContext,
        key: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringProctected {
        let key_value = key.as_string().unwrap();
        // resolve path to file system
        let test_module_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        // module key can start with ./ or ../
        let path = std::path::Path::new(test_module_dir).join(key_value.to_string());
        let module_path = std::fs::canonicalize(path).unwrap();

        JSStringProctected::from(module_path.to_str().unwrap())
    }

    #[module_fetch]
    fn module_loader_fetch(
        _ctx: JSContext,
        _key: JSValue,
        _attributes_value: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringProctected {
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

        JSStringProctected::from(file_content)
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
    fn test_inspectable_basic() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);
        assert_eq!(ctx.is_inspectable(), true);

        #[inspector_callback]
        fn inspector_callback(message: &str) {
            println!("Inspector message: {}", message);
        }

        ctx.set_inspector_callback(inspector_callback);
        assert!(ctx.inspector_is_connected());

        // Test Runtime.evaluate command
        let message = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "2 + 2"}}"#;
        ctx.inspector_send_message(message);

        // Properly disconnect before context is dropped
        ctx.inspector_disconnect();
        assert!(!ctx.inspector_is_connected());
    }

    #[test]
    fn test_inspector_comprehensive() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        #[inspector_callback]
        fn comprehensive_callback(message: &str) {
            println!("Inspector response: {}", message);
        }

        ctx.set_inspector_callback(comprehensive_callback);
        assert!(ctx.inspector_is_connected());

        // Test Runtime.evaluate for arithmetic
        let eval_message = r#"{"id": 10, "method": "Runtime.evaluate", "params": {"expression": "3 * 7"}}"#;
        ctx.inspector_send_message(eval_message);

        // Test Runtime.evaluate with string
        let string_eval = r#"{"id": 12, "method": "Runtime.evaluate", "params": {"expression": "'Hello' + ' World'"}}"#;
        ctx.inspector_send_message(string_eval);

        // Properly disconnect before context is dropped
        ctx.inspector_disconnect();
        assert!(!ctx.inspector_is_connected());
    }

    // #[test]
    // fn test_inspector_error_handling() {
    //     use std::time::Duration;

    //     static CALLBACK_MESSAGES_M: std::sync::Mutex<Vec<String>> =
    //         std::sync::Mutex::new(Vec::new());

    //     let ctx = JSContext::new();
    //     ctx.set_inspectable(true);

    //     #[inspector_callback]
    //     fn error_callback(message: &str) {
    //         println!("Error test response: {}", message);
    //         CALLBACK_MESSAGES_M
    //             .lock()
    //             .unwrap()
    //             .push(message.to_string());
    //     }

    //     ctx.set_inspector_callback(error_callback);

    //     // Test invalid JavaScript
    //     let invalid_js = r#"{"id": 20, "method": "Runtime.evaluate", "params": {"expression": "invalid javascript syntax ++"}}"#;
    //     ctx.inspector_send_message(invalid_js);
    //     std::thread::sleep(Duration::from_millis(200));

    //     let messages = CALLBACK_MESSAGES_M.lock().unwrap().clone();
    //     assert!(!messages.is_empty(), "Should receive error response");

    //     // Look for error indication
    //     let has_error = messages.iter().any(|msg| {
    //         msg.contains("\"id\":20")
    //             && (msg.contains("\"wasThrown\":true")
    //                 || msg.contains("error")
    //                 || msg.contains("exception"))
    //     });

    //     if !has_error {
    //         println!("Messages received: {:?}", messages);
    //     }

    //     JSContext::inspector_cleanup();
    // }

    #[test]
    fn test_inspector_debugger_workflow() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        #[inspector_callback]
        fn workflow_callback(message: &str) {
            println!("Debugger workflow: {}", message);
        }

        ctx.set_inspector_callback(workflow_callback);
        assert!(ctx.inspector_is_connected());

        // Test Debugger.enable - this previously caused hangs due to missing JSLockHolder
        // in JSInspectorSendMessage. The lock is required because Debugger.enable triggers:
        // 1. Heap iteration to find all source providers
        // 2. Code recompilation when debugger attaches
        // 3. Proper VM state management during these operations
        let enable_debugger = r#"{"id": 30, "method": "Debugger.enable", "params": {}}"#;
        ctx.inspector_send_message(enable_debugger);

        // Test Runtime.evaluate while debugger is enabled
        let eval_msg = r#"{"id": 31, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#;
        ctx.inspector_send_message(eval_msg);

        // Disable debugger - this triggers code recompilation via recompileAllJSFunctions()
        let disable_debugger =
            r#"{"id": 32, "method": "Debugger.disable", "params": {}}"#;
        ctx.inspector_send_message(disable_debugger);

        // Properly disconnect before context is dropped
        ctx.inspector_disconnect();
        assert!(!ctx.inspector_is_connected());
    }

    #[test]
    fn test_debugger_pause_and_resume() {
        // This test demonstrates the pause/resume workflow.
        // When Debugger.pause is called, the VM will pause on the next JavaScript statement.
        // The Debugger.paused event is fired when the VM actually stops.
        // Debugger.resume continues execution and fires Debugger.resumed event.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn pause_callback(message: &str) {
            println!("Pause/Resume test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(pause_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Request pause - VM will pause on next JS statement
        ctx.inspector_send_message(
            r#"{"id": 2, "method": "Debugger.pause", "params": {}}"#,
        );

        let messages = MESSAGES.lock().unwrap().clone();
        let has_pause_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        assert!(
            has_pause_response,
            "Should receive response for pause command"
        );

        // Resume execution (in case we're paused)
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.resume", "params": {}}"#,
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_breakpoint_workflow() {
        // This test demonstrates a complete breakpoint workflow:
        // 1. Enable debugger
        // 2. Load a script
        // 3. Set a breakpoint by URL
        // 4. Verify breakpoint was created and resolved
        // 5. Remove the breakpoint
        // 6. Disable debugger

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn bp_workflow_callback(message: &str) {
            println!("Breakpoint workflow: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(bp_workflow_callback);

        // Step 1: Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Step 2: Load a script with multiple lines for breakpoint testing
        let script = r#"
function add(a, b) {
    let result = a + b;
    return result;
}

function multiply(a, b) {
    let result = a * b;
    return result;
}

export { add, multiply };
"#;
        ctx.evaluate_module_from_source(script, "math-utils.js", None)
            .unwrap();

        // Step 3: Set breakpoint at line 2 (inside add function)
        ctx.inspector_send_message(
            r#"{
            "id": 2,
            "method": "Debugger.setBreakpointByUrl",
            "params": {
                "lineNumber": 2,
                "url": "math-utils.js",
                "columnNumber": 0
            }
        }"#,
        );

        // Step 4: Set another breakpoint at line 7 (inside multiply function)
        ctx.inspector_send_message(
            r#"{
            "id": 3,
            "method": "Debugger.setBreakpointByUrl",
            "params": {
                "lineNumber": 7,
                "url": "math-utils.js"
            }
        }"#,
        );

        let messages = MESSAGES.lock().unwrap().clone();

        // Verify scriptParsed event was received
        let has_script_parsed = messages
            .iter()
            .any(|m| m.contains("Debugger.scriptParsed") && m.contains("math-utils.js"));
        assert!(has_script_parsed, "Should receive scriptParsed event");

        // Verify breakpoints were created (look for breakpointId in responses)
        let bp1_response = messages
            .iter()
            .find(|m| m.contains("\"id\":2") && m.contains("breakpointId"));
        let bp2_response = messages
            .iter()
            .find(|m| m.contains("\"id\":3") && m.contains("breakpointId"));

        assert!(
            bp1_response.is_some(),
            "Should receive breakpoint 1 response with breakpointId"
        );
        assert!(
            bp2_response.is_some(),
            "Should receive breakpoint 2 response with breakpointId"
        );

        // Extract breakpointId from first breakpoint for removal
        if let Some(bp_msg) = bp1_response {
            if bp_msg.contains("math-utils.js:2") {
                // Step 5: Remove the first breakpoint
                ctx.inspector_send_message(
                    r#"{
                    "id": 4,
                    "method": "Debugger.removeBreakpoint",
                    "params": {"breakpointId": "math-utils.js:2:0"}
                }"#,
                );

                let updated_messages = MESSAGES.lock().unwrap().clone();
                let has_remove_response = updated_messages
                    .iter()
                    .any(|m| m.contains("\"id\":4") && m.contains("result"));
                assert!(
                    has_remove_response,
                    "Should receive response for removeBreakpoint"
                );
            }
        }

        // Step 6: Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 5, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_stepping_commands() {
        // This test verifies that stepping commands are accepted by the debugger.
        // Note: stepNext, stepOver, stepInto, stepOut require the debugger to be PAUSED.
        // When not paused, they return an error "Must be paused".
        // This test demonstrates the expected behavior.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn step_callback(message: &str) {
            println!("Stepping test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(step_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Try stepping commands while NOT paused - should get "Must be paused" error
        // This is expected behavior - stepping only works when VM is paused

        // stepNext - steps over the expression
        ctx.inspector_send_message(
            r#"{"id": 2, "method": "Debugger.stepNext", "params": {}}"#,
        );

        // stepOver - steps over the statement
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.stepOver", "params": {}}"#,
        );

        // stepInto - steps into function call
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.stepInto", "params": {}}"#,
        );

        // stepOut - steps out of current function
        ctx.inspector_send_message(
            r#"{"id": 5, "method": "Debugger.stepOut", "params": {}}"#,
        );

        let messages = MESSAGES.lock().unwrap().clone();

        // All stepping commands should return "Must be paused" error when not paused
        let step_next_response = messages.iter().find(|m| m.contains("\"id\":2"));
        let step_over_response = messages.iter().find(|m| m.contains("\"id\":3"));
        let step_into_response = messages.iter().find(|m| m.contains("\"id\":4"));
        let step_out_response = messages.iter().find(|m| m.contains("\"id\":5"));

        assert!(
            step_next_response.is_some(),
            "Should receive response for stepNext"
        );
        assert!(
            step_over_response.is_some(),
            "Should receive response for stepOver"
        );
        assert!(
            step_into_response.is_some(),
            "Should receive response for stepInto"
        );
        assert!(
            step_out_response.is_some(),
            "Should receive response for stepOut"
        );

        // Verify they return "Must be paused" error
        if let Some(msg) = step_next_response {
            let has_error = msg.contains("Must be paused") || msg.contains("error");
            println!("stepNext response: {}", msg);
            assert!(
                has_error,
                "stepNext should return 'Must be paused' error when not paused"
            );
        }

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 6, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_continue_to_location() {
        // continueToLocation continues execution until a specific location is reached.
        // Like stepping commands, it requires a paused state to work properly.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn continue_callback(message: &str) {
            println!("Continue to location test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(continue_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a script
        ctx.evaluate_module_from_source(
            "function test() { return 1; }\nexport default test;",
            "continue-test.js",
            None,
        )
        .unwrap();

        // Get the scriptId from scriptParsed event
        let messages = MESSAGES.lock().unwrap().clone();
        let script_parsed = messages.iter().find(|m| {
            m.contains("Debugger.scriptParsed") && m.contains("continue-test.js")
        });

        if let Some(parsed_msg) = script_parsed {
            if let Some(start) = parsed_msg.find("\"scriptId\":\"") {
                let rest = &parsed_msg[start + 12..];
                if let Some(end) = rest.find("\"") {
                    let script_id = &rest[..end];

                    // Try continueToLocation (will error since not paused)
                    let continue_cmd = format!(
                        r#"{{"id": 2, "method": "Debugger.continueToLocation", "params": {{"location": {{"scriptId": "{}", "lineNumber": 0, "columnNumber": 0}}}}}}"#,
                        script_id
                    );
                    ctx.inspector_send_message(&continue_cmd);

                    let updated_messages = MESSAGES.lock().unwrap().clone();
                    let has_response =
                        updated_messages.iter().any(|m| m.contains("\"id\":2"));
                    assert!(
                        has_response,
                        "Should receive response for continueToLocation"
                    );
                }
            }
        }

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_breakpoint_with_condition() {
        // Breakpoints can have conditions - they only pause if the condition evaluates to true.
        // This is set via the "options" parameter with a "condition" field.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn condition_callback(message: &str) {
            println!("Conditional breakpoint test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(condition_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a script
        ctx.evaluate_module_from_source(
            "function loop(n) { for(let i=0; i<n; i++) { console.log(i); } }\nexport default loop;",
            "conditional-bp.js",
            None,
        )
        .unwrap();

        // Set a conditional breakpoint - only pause when i === 5
        ctx.inspector_send_message(
            r#"{
            "id": 2,
            "method": "Debugger.setBreakpointByUrl",
            "params": {
                "lineNumber": 0,
                "url": "conditional-bp.js",
                "columnNumber": 35,
                "options": {
                    "condition": "i === 5"
                }
            }
        }"#,
        );

        let messages = MESSAGES.lock().unwrap().clone();
        let has_breakpoint = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("breakpointId"));
        assert!(has_breakpoint, "Should create conditional breakpoint");

        // Set a breakpoint with ignoreCount - skip first N hits
        ctx.inspector_send_message(
            r#"{
            "id": 3,
            "method": "Debugger.setBreakpointByUrl",
            "params": {
                "lineNumber": 0,
                "url": "conditional-bp.js",
                "options": {
                    "ignoreCount": 10
                }
            }
        }"#,
        );

        let updated_messages = MESSAGES.lock().unwrap().clone();
        let has_ignore_bp = updated_messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("breakpointId"));
        assert!(has_ignore_bp, "Should create breakpoint with ignoreCount");

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_get_breakpoint_locations() {
        // getBreakpointLocations returns valid breakpoint locations within a range.
        // This helps IDEs show where breakpoints can be set.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn locations_callback(message: &str) {
            println!("Breakpoint locations test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(locations_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a multi-line script
        let script = r#"function example() {
    let x = 1;
    let y = 2;
    let z = x + y;
    return z;
}
export default example;"#;
        ctx.evaluate_module_from_source(script, "locations-test.js", None)
            .unwrap();

        // Get scriptId
        let messages = MESSAGES.lock().unwrap().clone();
        let script_parsed = messages.iter().find(|m| {
            m.contains("Debugger.scriptParsed") && m.contains("locations-test.js")
        });

        if let Some(parsed_msg) = script_parsed {
            if let Some(start) = parsed_msg.find("\"scriptId\":\"") {
                let rest = &parsed_msg[start + 12..];
                if let Some(end) = rest.find("\"") {
                    let script_id = &rest[..end];

                    // Get breakpoint locations for lines 1-5
                    let get_locations = format!(
                        r#"{{"id": 2, "method": "Debugger.getBreakpointLocations", "params": {{"start": {{"scriptId": "{}", "lineNumber": 1}}, "end": {{"scriptId": "{}", "lineNumber": 5}}}}}}"#,
                        script_id, script_id
                    );
                    ctx.inspector_send_message(&get_locations);

                    let updated_messages = MESSAGES.lock().unwrap().clone();
                    let locations_response =
                        updated_messages.iter().find(|m| m.contains("\"id\":2"));

                    assert!(
                        locations_response.is_some(),
                        "Should receive breakpoint locations response"
                    );

                    if let Some(resp) = locations_response {
                        println!("Breakpoint locations: {}", resp);
                        // Response should contain "locations" array
                        let has_locations = resp.contains("locations");
                        assert!(has_locations, "Response should contain locations array");
                    }
                }
            }
        }

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_search_in_content() {
        // searchInContent searches for a string in a script's source.
        // Useful for "Find in Files" functionality.

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn search_callback(message: &str) {
            println!("Search in content test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(search_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a script with searchable content
        let script = r#"
// This is a comment about MAGIC_VALUE
const MAGIC_VALUE = 42;
function getMagicValue() {
    return MAGIC_VALUE;
}
export { MAGIC_VALUE, getMagicValue };
"#;
        ctx.evaluate_module_from_source(script, "search-test.js", None)
            .unwrap();

        // Get scriptId
        let messages = MESSAGES.lock().unwrap().clone();
        let script_parsed = messages.iter().find(|m| {
            m.contains("Debugger.scriptParsed") && m.contains("search-test.js")
        });

        if let Some(parsed_msg) = script_parsed {
            if let Some(start) = parsed_msg.find("\"scriptId\":\"") {
                let rest = &parsed_msg[start + 12..];
                if let Some(end) = rest.find("\"") {
                    let script_id = &rest[..end];

                    // Search for "MAGIC_VALUE" in the script
                    let search_cmd = format!(
                        r#"{{"id": 2, "method": "Debugger.searchInContent", "params": {{"scriptId": "{}", "query": "MAGIC_VALUE", "caseSensitive": true}}}}"#,
                        script_id
                    );
                    ctx.inspector_send_message(&search_cmd);

                    let updated_messages = MESSAGES.lock().unwrap().clone();
                    let search_response =
                        updated_messages.iter().find(|m| m.contains("\"id\":2"));

                    assert!(search_response.is_some(), "Should receive search response");

                    if let Some(resp) = search_response {
                        println!("Search results: {}", resp);
                        // Should find matches
                        let has_result = resp.contains("result");
                        assert!(has_result, "Response should contain result");
                    }

                    // Search with regex
                    let regex_search = format!(
                        r#"{{"id": 3, "method": "Debugger.searchInContent", "params": {{"scriptId": "{}", "query": "MAGIC.*", "isRegex": true}}}}"#,
                        script_id
                    );
                    ctx.inspector_send_message(&regex_search);
                }
            }
        }

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_set_breakpoint_by_url() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn breakpoint_callback(message: &str) {
            println!("Breakpoint test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(breakpoint_callback);

        // Enable debugger first
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a module so we have a script to set breakpoints on
        ctx.evaluate_module_from_source(
            "function testFunc() { return 42; }\nexport default testFunc;",
            "test-breakpoint.js",
            None,
        )
        .unwrap();

        // Set a breakpoint by URL
        let set_breakpoint = r#"{
            "id": 2,
            "method": "Debugger.setBreakpointByUrl",
            "params": {
                "lineNumber": 0,
                "url": "test-breakpoint.js",
                "columnNumber": 0
            }
        }"#;
        ctx.inspector_send_message(set_breakpoint);

        let messages = MESSAGES.lock().unwrap().clone();

        // Should have received responses including breakpoint creation
        assert!(!messages.is_empty(), "Should receive inspector messages");

        // Check for breakpointId in response
        let has_breakpoint_response = messages.iter().any(|m| m.contains("breakpointId"));
        assert!(
            has_breakpoint_response,
            "Should receive breakpoint response with breakpointId"
        );

        // Remove the breakpoint
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.removeBreakpoint", "params": {"breakpointId": "test-breakpoint.js:0:0"}}"#,
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_pause_on_exceptions() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn exception_callback(message: &str) {
            println!("Exception test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(exception_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Test setPauseOnExceptions with different states
        // State can be: "none", "uncaught", "all"

        // Set to pause on all exceptions
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setPauseOnExceptions", "params": {"state": "all"}}"#);

        // Set to pause on uncaught exceptions only
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setPauseOnExceptions", "params": {"state": "uncaught"}}"#);

        // Disable pause on exceptions
        ctx.inspector_send_message(r#"{"id": 4, "method": "Debugger.setPauseOnExceptions", "params": {"state": "none"}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        // Should have responses for all commands
        let has_response_2 = messages.iter().any(|m| m.contains("\"id\":2"));
        let has_response_3 = messages.iter().any(|m| m.contains("\"id\":3"));
        let has_response_4 = messages.iter().any(|m| m.contains("\"id\":4"));

        assert!(
            has_response_2,
            "Should receive response for setPauseOnExceptions (all)"
        );
        assert!(
            has_response_3,
            "Should receive response for setPauseOnExceptions (uncaught)"
        );
        assert!(
            has_response_4,
            "Should receive response for setPauseOnExceptions (none)"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 5, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_set_breakpoints_active() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn active_callback(message: &str) {
            println!("Breakpoints active test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(active_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Deactivate all breakpoints
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setBreakpointsActive", "params": {"active": false}}"#);

        // Reactivate all breakpoints
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setBreakpointsActive", "params": {"active": true}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        let has_deactivate_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        let has_activate_response = messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("result"));

        assert!(
            has_deactivate_response,
            "Should receive response for deactivating breakpoints"
        );
        assert!(
            has_activate_response,
            "Should receive response for activating breakpoints"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_script_parsed_event() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn script_callback(message: &str) {
            println!("Script parsed test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(script_callback);

        // Enable debugger - this should trigger scriptParsed events for existing scripts
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Evaluate a new script - should trigger scriptParsed event
        ctx.inspector_send_message(r#"{"id": 2, "method": "Runtime.evaluate", "params": {"expression": "function newFunc() { return 'hello'; }"}}"#);

        // Load a module - should trigger scriptParsed event
        ctx.evaluate_module_from_source(
            "export const value = 123;",
            "parsed-test.js",
            None,
        )
        .unwrap();

        let messages = MESSAGES.lock().unwrap().clone();

        // Check for scriptParsed events
        let script_parsed_count = messages
            .iter()
            .filter(|m| m.contains("Debugger.scriptParsed"))
            .count();
        println!("Received {} scriptParsed events", script_parsed_count);

        assert!(
            script_parsed_count > 0,
            "Should receive at least one scriptParsed event"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_get_script_source() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn source_callback(message: &str) {
            println!("Script source test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(source_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Load a module to have a script with known content
        let script_content = "export function getAnswer() { return 42; }";
        ctx.evaluate_module_from_source(script_content, "source-test.js", None)
            .unwrap();

        let messages = MESSAGES.lock().unwrap().clone();

        // Find scriptId from scriptParsed event
        let script_parsed = messages.iter().find(|m| {
            m.contains("Debugger.scriptParsed") && m.contains("source-test.js")
        });

        if let Some(parsed_msg) = script_parsed {
            // Extract scriptId (simplified - in real code you'd parse JSON properly)
            if let Some(start) = parsed_msg.find("\"scriptId\":\"") {
                let rest = &parsed_msg[start + 12..];
                if let Some(end) = rest.find("\"") {
                    let script_id = &rest[..end];
                    println!("Found scriptId: {}", script_id);

                    // Request script source
                    let get_source = format!(
                        r#"{{"id": 2, "method": "Debugger.getScriptSource", "params": {{"scriptId": "{}"}}}}"#,
                        script_id
                    );
                    ctx.inspector_send_message(&get_source);

                    let updated_messages = MESSAGES.lock().unwrap().clone();
                    let has_source_response = updated_messages
                        .iter()
                        .any(|m| m.contains("\"id\":2") && m.contains("scriptSource"));
                    assert!(has_source_response, "Should receive script source response");
                }
            }
        }

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_async_stack_trace_depth() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn async_callback(message: &str) {
            println!("Async stack trace test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(async_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Set async stack trace depth
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setAsyncStackTraceDepth", "params": {"depth": 10}}"#);

        // Disable async stack traces
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setAsyncStackTraceDepth", "params": {"depth": 0}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        let has_depth_10_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        let has_depth_0_response = messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("result"));

        assert!(
            has_depth_10_response,
            "Should receive response for setting depth to 10"
        );
        assert!(
            has_depth_0_response,
            "Should receive response for setting depth to 0"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_pause_on_assertions() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn assertion_callback(message: &str) {
            println!("Pause on assertions test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(assertion_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Enable pause on assertions (console.assert)
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setPauseOnAssertions", "params": {"enabled": true}}"#);

        // Disable pause on assertions
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setPauseOnAssertions", "params": {"enabled": false}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        let has_enable_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        let has_disable_response = messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("result"));

        assert!(
            has_enable_response,
            "Should receive response for enabling pause on assertions"
        );
        assert!(
            has_disable_response,
            "Should receive response for disabling pause on assertions"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_symbolic_breakpoint() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn symbolic_callback(message: &str) {
            println!("Symbolic breakpoint test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(symbolic_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Add a symbolic breakpoint - pauses when function named "myFunction" is called
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.addSymbolicBreakpoint", "params": {"symbol": "myFunction", "caseSensitive": true}}"#);

        // Add a regex symbolic breakpoint
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.addSymbolicBreakpoint", "params": {"symbol": "test.*", "isRegex": true}}"#);

        // Remove symbolic breakpoints
        ctx.inspector_send_message(r#"{"id": 4, "method": "Debugger.removeSymbolicBreakpoint", "params": {"symbol": "myFunction", "caseSensitive": true}}"#);
        ctx.inspector_send_message(r#"{"id": 5, "method": "Debugger.removeSymbolicBreakpoint", "params": {"symbol": "test.*", "isRegex": true}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        // Verify we got responses (even if just empty result objects)
        let has_add_response = messages.iter().any(|m| m.contains("\"id\":2"));
        let has_remove_response = messages.iter().any(|m| m.contains("\"id\":4"));

        assert!(
            has_add_response,
            "Should receive response for adding symbolic breakpoint"
        );
        assert!(
            has_remove_response,
            "Should receive response for removing symbolic breakpoint"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 6, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_pause_on_debugger_statements() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn debugger_stmt_callback(message: &str) {
            println!("Debugger statements test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(debugger_stmt_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Enable pause on debugger statements
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setPauseOnDebuggerStatements", "params": {"enabled": true}}"#);

        // Disable pause on debugger statements
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setPauseOnDebuggerStatements", "params": {"enabled": false}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        let has_enable_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        let has_disable_response = messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("result"));

        assert!(
            has_enable_response,
            "Should receive response for enabling pause on debugger statements"
        );
        assert!(
            has_disable_response,
            "Should receive response for disabling pause on debugger statements"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_debugger_blackbox_url() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        static MESSAGES: std::sync::Mutex<Vec<String>> =
            std::sync::Mutex::new(Vec::new());
        MESSAGES.lock().unwrap().clear();

        #[inspector_callback]
        fn blackbox_callback(message: &str) {
            println!("Blackbox URL test: {}", message);
            MESSAGES.lock().unwrap().push(message.to_string());
        }

        ctx.set_inspector_callback(blackbox_callback);

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Blackbox a specific URL (ignore it when debugging)
        ctx.inspector_send_message(r#"{"id": 2, "method": "Debugger.setShouldBlackboxURL", "params": {"url": "vendor.js", "shouldBlackbox": true}}"#);

        // Blackbox URLs matching a regex pattern
        ctx.inspector_send_message(r#"{"id": 3, "method": "Debugger.setShouldBlackboxURL", "params": {"url": "node_modules/.*", "shouldBlackbox": true, "isRegex": true}}"#);

        // Unblackbox a URL
        ctx.inspector_send_message(r#"{"id": 4, "method": "Debugger.setShouldBlackboxURL", "params": {"url": "vendor.js", "shouldBlackbox": false}}"#);

        let messages = MESSAGES.lock().unwrap().clone();

        let has_blackbox_response = messages
            .iter()
            .any(|m| m.contains("\"id\":2") && m.contains("result"));
        let has_regex_response = messages
            .iter()
            .any(|m| m.contains("\"id\":3") && m.contains("result"));
        let has_unblackbox_response = messages
            .iter()
            .any(|m| m.contains("\"id\":4") && m.contains("result"));

        assert!(
            has_blackbox_response,
            "Should receive response for blackboxing URL"
        );
        assert!(
            has_regex_response,
            "Should receive response for blackboxing regex URL"
        );
        assert!(
            has_unblackbox_response,
            "Should receive response for unblackboxing URL"
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 5, "method": "Debugger.disable", "params": {}}"#,
        );
        ctx.inspector_disconnect();
    }

    // #[test]
    // fn test_inspector_multiple_evaluations() {
    //     use std::time::Duration;

    //     clear_test_messages();

    //     let ctx = JSContext::new();
    //     ctx.set_inspectable(true);

    //     #[inspector_callback]
    //     fn multi_callback(message: &str) {
    //         println!("Multi eval: {}", message);
    //         add_test_message(message);
    //     }

    //     ctx.set_inspector_callback(multi_callback);

    //     // Multiple evaluations to test robustness
    //     let evaluations = vec![
    //         (40, "1 + 1", "2"),
    //         (41, "Math.PI", "3.14"),
    //         (42, "typeof 'hello'", "string"),
    //         (43, "[1,2,3].length", "3"),
    //     ];

    //     for (id, expr, expected) in evaluations {
    //         let message = format!(
    //             r#"{{"id": {}, "method": "Runtime.evaluate", "params": {{"expression": "{}"}}}}"#,
    //             id, expr
    //         );
    //         ctx.inspector_send_message(&message);
    //         std::thread::sleep(Duration::from_millis(150));

    //         let messages = get_test_messages();
    //         let found_response = messages.iter().any(|msg| {
    //             msg.contains(&format!("\"id\":{}", id)) && msg.contains(expected)
    //         });

    //         if !found_response {
    //             println!(
    //                 "Failed to find expected response for {}: {}",
    //                 expr, expected
    //             );
    //             println!("Current messages: {:?}", messages);
    //         }
    //     }

    //     let final_messages = get_test_messages();
    //     assert!(
    //         final_messages.len() >= 4,
    //         "Should have received at least 4 responses"
    //     );

    //     JSContext::inspector_cleanup();
    // }

    // #[test]
    // fn test_inspector_advanced_debugging() {
    //     use std::time::Duration;

    //     clear_test_messages();

    //     let ctx = JSContext::new();
    //     ctx.set_inspectable(true);

    //     #[inspector_callback]
    //     fn advanced_callback(message: &str) {
    //         println!("Advanced debug: {}", message);
    //         add_test_message(message);
    //     }

    //     ctx.set_inspector_callback(advanced_callback);

    //     // Enable both debugger and runtime
    //     let enable_debugger = r#"{"id": 50, "method": "Debugger.enable", "params": {}}"#;
    //     ctx.inspector_send_message(enable_debugger);
    //     std::thread::sleep(Duration::from_millis(100));

    //     let enable_runtime = r#"{"id": 51, "method": "Runtime.enable", "params": {}}"#;
    //     ctx.inspector_send_message(enable_runtime);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Create a function and execute it
    //     let create_function = r#"{"id": 52, "method": "Runtime.evaluate", "params": {"expression": "function debugTest(x) { var result = x * 2; return result + 1; }"}}"#;
    //     ctx.inspector_send_message(create_function);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Call the function
    //     let call_function = r#"{"id": 53, "method": "Runtime.evaluate", "params": {"expression": "debugTest(5)"}}"#;
    //     ctx.inspector_send_message(call_function);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Test console API
    //     let console_test = r#"{"id": 54, "method": "Runtime.evaluate", "params": {"expression": "console.log('Debug test message'); 42"}}"#;
    //     ctx.inspector_send_message(console_test);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Get global object properties
    //     let global_props = r#"{"id": 55, "method": "Runtime.evaluate", "params": {"expression": "Object.keys(this).slice(0, 5)"}}"#;
    //     ctx.inspector_send_message(global_props);
    //     std::thread::sleep(Duration::from_millis(100));

    //     let messages = get_test_messages();

    //     // Validate responses
    //     assert!(
    //         validate_response(&messages, 50, "result"),
    //         "Debugger should enable"
    //     );
    //     assert!(
    //         validate_response(&messages, 51, "result"),
    //         "Runtime should enable"
    //     );
    //     assert!(
    //         validate_response(&messages, 52, "result"),
    //         "Function should be created"
    //     );
    //     assert!(
    //         validate_response(&messages, 53, "\"value\":11"),
    //         "Function should return 11 (5*2+1)"
    //     );
    //     assert!(
    //         validate_response(&messages, 54, "\"value\":42"),
    //         "Console test should return 42"
    //     );
    //     assert!(
    //         validate_response(&messages, 55, "array"),
    //         "Should get global properties array"
    //     );

    //     println!(
    //         "Advanced debugging test completed with {} messages",
    //         messages.len()
    //     );
    //     for (i, msg) in messages.iter().enumerate() {
    //         if msg.len() > 200 {
    //             println!("Message {}: {}...", i + 1, &msg[..200]);
    //         } else {
    //             println!("Message {}: {}", i + 1, msg);
    //         }
    //     }

    //     JSContext::inspector_cleanup();
    // }

    // #[test]
    // fn test_inspector_breakpoint_with_execution() {
    //     use std::time::Duration;

    //     clear_test_messages();

    //     let ctx = JSContext::new();
    //     ctx.set_inspectable(true);

    //     #[inspector_callback]
    //     fn breakpoint_execution_callback(message: &str) {
    //         println!("Breakpoint execution: {}", message);
    //         add_test_message(message);
    //     }

    //     ctx.set_inspector_callback(breakpoint_execution_callback);

    //     // Enable debugger
    //     let enable = r#"{"id": 60, "method": "Debugger.enable", "params": {}}"#;
    //     ctx.inspector_send_message(enable);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Set up a more complex script that we can debug
    //     let script_setup = r#"{"id": 61, "method": "Runtime.evaluate", "params": {"expression": "var counter = 0; function increment() { counter++; return counter; }"}}"#;
    //     ctx.inspector_send_message(script_setup);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Try to set a breakpoint (though it may not hit without proper source mapping)
    //     let set_bp = r#"{"id": 62, "method": "Debugger.setBreakpointByUrl", "params": {"lineNumber": 1, "url": "test-script.js"}}"#;
    //     ctx.inspector_send_message(set_bp);
    //     std::thread::sleep(Duration::from_millis(100));

    //     // Execute the function multiple times
    //     for i in 0..3 {
    //         let call_increment = format!(
    //             r#"{{"id": {}, "method": "Runtime.evaluate", "params": {{"expression": "increment()"}}}}"#,
    //             63 + i
    //         );
    //         ctx.inspector_send_message(&call_increment);
    //         std::thread::sleep(Duration::from_millis(50));
    //     }

    //     // Check the final counter value
    //     let check_counter = r#"{"id": 66, "method": "Runtime.evaluate", "params": {"expression": "counter"}}"#;
    //     ctx.inspector_send_message(check_counter);
    //     std::thread::sleep(Duration::from_millis(100));

    //     let messages = get_test_messages();

    //     // Validate that our script execution worked
    //     assert!(
    //         validate_response(&messages, 66, "\"value\":3"),
    //         "Counter should be 3 after 3 increments"
    //     );

    //     // Check that we got multiple function call responses
    //     let increment_responses = messages
    //         .iter()
    //         .filter(|msg| (63..66).any(|id| msg.contains(&format!("\"id\":{}", id))))
    //         .count();
    //     assert!(
    //         increment_responses >= 3,
    //         "Should have responses for all increment calls"
    //     );

    //     println!("Breakpoint execution test completed");
    //     println!("Found {} increment responses", increment_responses);

    //     JSContext::inspector_cleanup();
    // }

    #[test]
    fn test_virtual_module() {
        let ctx = JSContext::new();
        let keys = &[JSStringProctected::from("@rust-jsc")];
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
        let result = ctx.evaluate_module_from_source(
            r"
            import lib from '@rust-jsc';
            globalThis.lib = lib;
        ",
            "virtual_module.js",
            None,
        );
        assert!(result.is_ok());

        let result = ctx.evaluate_script("lib.name", None);

        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value.as_string().unwrap(), "John Doe");

        let result = ctx.evaluate_module_from_source(
            r"
            import { name } from '@rust-jsc';
            globalThis.name = name;
        ",
            "virtual_module.js",
            None,
        );
        assert!(result.is_ok());

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");
    }

    #[test]
    fn test_virtual_module_no_default() {
        let ctx = JSContext::new();
        let keys = &[JSStringProctected::from("@rust-jsc")];
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

        let result = ctx.evaluate_module_from_source(
            r"
            import { name } from '@rust-jsc';
            globalThis.name = name;
        ",
            "virtual_module.js",
            None,
        );

        assert!(result.is_ok());

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");

        let result = ctx.evaluate_module_from_source(
            r"
            import lib from '@rust-jsc';
            globalThis.lib = lib;
        ",
            "virtual_module.js",
            None,
        );
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

    #[test]
    fn test_set_uncaught_exception_handler() {
        let ctx = JSContext::new();
        ctx.set_uncaught_exception_handler(Some(uncaught_exception_handler));

        let script =
            "function throwError() { throw new Error('Error thrown'); }; throwError();";
        let result = ctx.evaluate_module_from_source(
            script,
            "uncaught_exception_handler.js",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_uncaught_exception_at_event_loop_callback() {
        let ctx = JSContext::new();
        ctx.set_uncaught_exception_at_event_loop_callback(Some(
            uncaught_exception_event_loop,
        ));
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
