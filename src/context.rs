use crate::{
    not_send_or_sync, JSClass, JSContext, JSContextGroup, JSError, JSGlobalContext,
    JSObject, JSResult, JSString, JSStringProctected, JSValue, OwnedJSContextGroup,
    PrivateDataDropStatus, PrivateDataSetStatus, PrivateDataTakeResult,
    PrivateDataWrapper, TypedJSContext,
};
use rust_jsc_sys::{
    InspectorMessageCallback, InspectorPauseEventCallback, JSAPIModuleLoader,
    JSCheckScriptSyntax, JSContextGetGlobalContext, JSContextGetGlobalObject,
    JSContextGetGroup, JSContextGetSharedData, JSContextGroupCreate, JSContextGroupRef,
    JSContextGroupRelease, JSContextGroupRetain, JSContextRef, JSContextSetSharedData,
    JSEvaluateScript, JSGarbageCollect, JSGetMemoryUsageStatistics,
    JSGlobalContextCopyName, JSGlobalContextCreate, JSGlobalContextCreateInGroup,
    JSGlobalContextIsInspectable, JSGlobalContextRef, JSGlobalContextRelease,
    JSGlobalContextRetain, JSGlobalContextSetInspectable, JSGlobalContextSetName,
    JSGlobalContextSetUncaughtExceptionAtEventLoopCallback,
    JSGlobalContextSetUncaughtExceptionHandler,
    JSGlobalContextSetUnhandledRejectionCallback, JSInspectorDisconnect,
    JSInspectorIsConnected, JSInspectorSendMessage, JSInspectorSetCallback,
    JSInspectorSetPauseEventCallback, JSLinkAndEvaluateModule, JSLoadAndEvaluateModule,
    JSLoadAndEvaluateModuleFromSource, JSLoadModule, JSLoadModuleFromSource,
    JSSetAPIModuleLoader, JSSetSyntheticModuleKeys, JSStringRef,
    JSUncaughtExceptionAtEventLoop, JSUncaughtExceptionHandler, JSValueRef,
};
use std::ffi::CString;

impl JSContextGroup {
    /// Creates a borrowed JavaScriptCore context-group view from a raw group.
    ///
    /// The returned [`JSContextGroup`] does not retain or release the raw
    /// `JSContextGroupRef`. Use [`JSContextGroup::retain`] when Rust must own a
    /// group reference beyond the owner that provided this raw pointer.
    ///
    /// # Safety
    /// `context_group` must be a valid `JSContextGroupRef` and must remain
    /// alive while the returned borrowed view is used.
    pub unsafe fn borrowed(context_group: JSContextGroupRef) -> Self {
        Self::from_ref(context_group)
    }

    pub(crate) fn from_ref(context_group: JSContextGroupRef) -> Self {
        Self {
            context_group,
            _not_send_or_sync: not_send_or_sync(),
        }
    }

    pub fn new() -> OwnedJSContextGroup {
        OwnedJSContextGroup::new()
    }

    pub fn retain(&self) -> OwnedJSContextGroup {
        let context_group = unsafe { JSContextGroupRetain(self.context_group) };
        OwnedJSContextGroup {
            inner: Self::from_ref(context_group),
        }
    }

    pub fn new_context(&self) -> JSGlobalContext {
        let ctx = unsafe {
            JSGlobalContextCreateInGroup(self.context_group, std::ptr::null_mut())
        };
        JSGlobalContext::from_owned_ref(ctx)
    }

    pub fn new_context_with_class(&self, class: &JSClass) -> JSGlobalContext {
        let ctx =
            unsafe { JSGlobalContextCreateInGroup(self.context_group, class.inner) };
        JSGlobalContext::from_owned_ref(ctx)
    }
}

impl OwnedJSContextGroup {
    /// Creates a new owned JavaScript context group.
    ///
    /// JavaScriptCore ties deferred group work to the run loop of the creating
    /// thread. Use contexts and values from the group on one thread unless you
    /// provide external synchronization.
    pub fn new() -> Self {
        let context_group = unsafe { JSContextGroupCreate() };
        Self {
            inner: JSContextGroup::from_ref(context_group),
        }
    }

    pub fn as_context_group(&self) -> &JSContextGroup {
        &self.inner
    }
}

impl std::ops::Deref for OwnedJSContextGroup {
    type Target = JSContextGroup;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for OwnedJSContextGroup {
    fn drop(&mut self) {
        unsafe {
            JSContextGroupRelease(self.inner.context_group);
        }
    }
}

impl Default for OwnedJSContextGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for JSContextGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSContextGroup").finish()
    }
}

impl std::fmt::Debug for OwnedJSContextGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedJSContextGroup").finish()
    }
}

/// Debugger pause-loop events emitted by JavaScriptCore while debugging.
///
/// - `Paused`: debugger just entered paused state (breakpoint, `debugger;`, etc.)
/// - `Resumed`: debugger just resumed execution
/// - `Tick`: called repeatedly while the debugger is paused (nested run loop)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPauseEvent {
    Paused,
    Resumed,
    Tick,
}

impl JSContext {
    /// Creates a borrowed JavaScriptCore context view from a raw context.
    ///
    /// The returned [`JSContext`] does not retain or release the underlying
    /// global context. This is the correct wrapper for JavaScriptCore callback
    /// arguments. Use [`JSContext::retain`] or [`JSGlobalContext::retain_from_raw`]
    /// when Rust needs an owned context handle.
    ///
    /// # Safety
    /// `context` must be a valid `JSContextRef` and its global context must
    /// remain alive while the returned borrowed view is used.
    pub unsafe fn borrowed(context: JSContextRef) -> Self {
        Self::from_ref(context)
    }

    pub(crate) fn from_ref(context: JSContextRef) -> Self {
        let global_context = unsafe { JSContextGetGlobalContext(context) };
        Self::from_global_ref(global_context)
    }

    pub(crate) fn from_global_ref(inner: JSGlobalContextRef) -> Self {
        Self {
            inner,
            _not_send_or_sync: not_send_or_sync(),
        }
    }

    /// Creates a new owned JavaScript global context.
    ///
    /// The returned [`JSGlobalContext`] releases the underlying
    /// `JSGlobalContextRef` in `Drop`. Borrowed callback contexts are represented
    /// by [`JSContext`] and do not release the context.
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ```
    pub fn new() -> JSGlobalContext {
        JSGlobalContext::new()
    }

    pub fn new_with(class: &JSClass) -> JSGlobalContext {
        JSGlobalContext::new_with(class)
    }

    pub fn retain(&self) -> JSGlobalContext {
        let ctx = unsafe { JSGlobalContextRetain(self.inner) };
        JSGlobalContext::from_owned_ref(ctx)
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
        JSContextGroup::from_ref(group)
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
    /// On rebased WebKit builds this returns the JavaScript `Promise` created by
    /// JavaScriptCore's async module evaluation path.
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
    /// ```ignore
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let promise = ctx.link_and_evaluate_module("test")?;
    /// assert!(promise.is_object());
    /// ```
    ///
    /// # Returns
    ///
    /// A `JSValue` containing the evaluation `Promise`.
    pub fn link_and_evaluate_module(&self, key: &str) -> JSResult<JSValue> {
        let module_key: JSString = key.into();
        let result = unsafe { JSLinkAndEvaluateModule(self.inner, module_key.inner) };

        if result.is_null() {
            return Err(JSError::new_typ(
                self,
                format!("failed to link and evaluate module `{key}`"),
            )?);
        }

        Ok(JSValue::new(result, self.inner))
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
    pub fn load_module_from_source(
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
    /// The keys are used to identify synthetic modules when loading modules.
    /// The strings remain caller-owned; JavaScriptCore copies the key text and
    /// does not retain or release the provided `JSStringRef` values.
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

        if result.is_null() {
            return Err(JSError::from_message(
                self,
                "script evaluation returned a null JavaScriptCore value",
            ));
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
    pub fn set_inspector_callback(&self, callback: InspectorMessageCallback) {
        unsafe {
            JSInspectorSetCallback(self.inner, callback);
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

    /// Registers a pause-event callback.
    ///
    /// # Arguments
    /// - `callback`: A callback function.
    pub fn set_inspector_pause_event_callback(
        &self,
        callback: InspectorPauseEventCallback,
    ) {
        unsafe {
            JSInspectorSetPauseEventCallback(self.inner, callback);
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
    /// The data is wrapped in a type-safe container that tracks the original type,
    /// preventing type confusion when retrieved with [`get_shared_data`].
    ///
    /// # Arguments
    /// - `data`: The data to store. Accepts any `'static` type.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(10i32);
    /// ```
    ///
    /// # Note
    /// This method only stores into an empty slot. If shared data already
    /// exists, the context is left unchanged and
    /// [`PrivateDataSetStatus::AlreadySet`] is returned. Use
    /// [`replace_shared_data`] when you explicitly own the current data and want
    /// to replace it.
    pub fn set_shared_data<T: 'static>(&self, data: T) -> PrivateDataSetStatus {
        let current = unsafe { JSContextGetSharedData(self.inner) };
        if !current.is_null() {
            return PrivateDataSetStatus::AlreadySet;
        }

        let ptr = PrivateDataWrapper::into_raw(data);
        unsafe { JSContextSetSharedData(self.inner, ptr) };
        PrivateDataSetStatus::Set
    }

    /// Replaces shared data, dropping the existing Rust-owned allocation first.
    ///
    /// # Safety
    /// The caller must ensure that the existing shared-data pointer, if any, was
    /// created by [`PrivateDataWrapper::into_raw`] and that no references into
    /// the existing allocation are alive.
    pub unsafe fn replace_shared_data<T: 'static>(
        &self,
        data: T,
    ) -> PrivateDataSetStatus {
        let current = unsafe { JSContextGetSharedData(self.inner) };
        let status = if current.is_null() {
            PrivateDataSetStatus::Set
        } else {
            unsafe { PrivateDataWrapper::drop_erased(current) };
            unsafe { JSContextSetSharedData(self.inner, std::ptr::null_mut()) };
            PrivateDataSetStatus::Replaced
        };

        let ptr = PrivateDataWrapper::into_raw(data);
        unsafe { JSContextSetSharedData(self.inner, ptr) };
        status
    }

    /// Gets shared data for a context as an immutable reference.
    ///
    /// Returns `None` if no data is set or if `T` does not match the type
    /// that was originally stored with [`set_shared_data`].
    ///
    /// # Examples
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(10i32);
    /// let shared_data = ctx.get_shared_data::<i32>().unwrap();
    /// assert_eq!(*shared_data, 10);
    /// ```
    ///
    /// # Type Safety
    /// Unlike raw pointer casts, this method uses `TypeId`-based checking.
    /// Requesting the wrong type returns `None` instead of causing UB:
    /// ```
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(String::from("hello"));
    /// assert!(ctx.get_shared_data::<u64>().is_none()); // wrong type → None
    /// assert!(ctx.get_shared_data::<String>().is_some()); // correct type → Some
    /// ```
    pub fn get_shared_data<T: 'static>(&self) -> Option<&T> {
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        unsafe { PrivateDataWrapper::downcast_ref(data_ptr) }
    }

    /// Gets shared data for a context as a mutable reference.
    ///
    /// Returns `None` if no data is set or if `T` does not match the type
    /// that was originally stored with [`set_shared_data`].
    ///
    /// # Safety
    ///
    /// The caller **must** guarantee that:
    ///
    /// 1. No other `&T` or `&mut T` references obtained from [`get_shared_data`]
    ///    or this method are alive at the time of the call.
    /// 2. The returned `&mut T` is not held simultaneously with any other
    ///    reference to the same data.
    ///
    /// Violating these rules creates **aliased mutable references**, which is
    /// instant undefined behavior — the compiler may reorder reads/writes,
    /// cache stale values, or miscompile the program.
    ///
    /// # Recommended alternative
    ///
    /// For safe interior mutability, store a [`std::cell::RefCell<T>`] and use
    /// [`get_shared_data`] instead:
    ///
    /// ```no_run
    /// use rust_jsc::JSContext;
    /// use std::cell::RefCell;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(RefCell::new(10i32));
    ///
    /// // Safe runtime borrow checking — panics on double mutable borrow
    /// let cell = ctx.get_shared_data::<RefCell<i32>>().unwrap();
    /// *cell.borrow_mut() = 20;
    /// assert_eq!(*cell.borrow(), 20);
    /// ```
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(10i32);
    /// // SAFETY: no other references to the shared data exist.
    /// if let Some(data) = unsafe { ctx.get_shared_data_mut::<i32>() } {
    ///     *data = 20;
    /// }
    /// assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 20);
    /// ```
    pub unsafe fn get_shared_data_mut<T: 'static>(&self) -> Option<&mut T> {
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        unsafe { PrivateDataWrapper::downcast_mut(data_ptr) }
    }

    /// Takes ownership of the shared data, removing it from the context.
    ///
    /// Returns `None` if no data is set or if `T` does not match the stored type.
    /// if T does not match, the data remains in place and is not freed,
    /// so it can still be retrieved with the correct type.
    ///
    /// # Safety
    /// This function is unsafe because `JSContext` is a handle that can be cloned.
    /// If other references to the shared data exist (e.g. obtained via `get_shared_data`
    /// from a cloned handle), calling this method will free the memory they point to,
    /// causing Undefined Behavior (Use-After-Free).
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(42i32);
    /// let data: i32 = unsafe { ctx.take_shared_data::<i32>() }.unwrap();
    /// assert_eq!(data, 42);
    /// assert!(ctx.get_shared_data::<i32>().is_none()); // data has been removed
    /// ```
    ///
    /// # Safety Note
    /// The caller must ensure that the type `T` matches the type of the data currently
    /// stored in the context. If the type does not match, this method will return `None`
    /// and leave the data in place.
    pub unsafe fn take_shared_data<T: 'static>(&self) -> PrivateDataTakeResult<T> {
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        // Only take ownership (and clear the JSC pointer) if the type matches.
        // On type mismatch the data stays in place — nothing is freed or lost.
        let result = unsafe { PrivateDataWrapper::take(data_ptr) };
        if result.is_taken() {
            unsafe { JSContextSetSharedData(self.inner, std::ptr::null_mut()) };
        }
        result
    }

    /// Drops the shared data without reclaiming ownership.
    /// This is useful for cleaning up data when the context is being dropped, without needing to take ownership of it.
    /// After this call, the context's shared data pointer is cleared only when
    /// the stored type matches `T`.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(String::from("temporary data"));
    /// unsafe { ctx.drop_shared_data::<String>() }; // Clean up without taking ownership
    /// assert!(ctx.get_shared_data::<String>().is_none()); // data has been removed
    /// ```
    /// # Safety
    /// The caller must ensure that no references into the shared data are alive.
    /// If the type does not match, this method returns
    /// [`PrivateDataDropStatus::TypeMismatch`] and leaves the data pointer intact.
    pub unsafe fn drop_shared_data<T: 'static>(&self) -> PrivateDataDropStatus {
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        let status = unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
        if status.is_dropped() {
            unsafe { JSContextSetSharedData(self.inner, std::ptr::null_mut()) };
        }
        status
    }
}

impl std::fmt::Debug for JSContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSContext").finish()
    }
}

impl JSGlobalContext {
    pub(crate) fn from_owned_ref(inner: JSGlobalContextRef) -> Self {
        Self {
            inner: JSContext::from_global_ref(inner),
        }
    }

    /// Creates a new owned JavaScript global context.
    pub fn new() -> Self {
        let ctx = unsafe { JSGlobalContextCreate(std::ptr::null_mut()) };
        Self::from_owned_ref(ctx)
    }

    /// Creates a new owned JavaScript global context with a custom global class.
    pub fn new_with(class: &JSClass) -> Self {
        let ctx = unsafe { JSGlobalContextCreate(class.inner) };
        Self::from_owned_ref(ctx)
    }

    /// Retains a borrowed raw global context and returns an owned RAII handle.
    ///
    /// # Safety
    /// The raw pointer must be a valid `JSGlobalContextRef`.
    pub unsafe fn retain_from_raw(ctx: JSGlobalContextRef) -> Self {
        let ctx = unsafe { JSGlobalContextRetain(ctx) };
        Self::from_owned_ref(ctx)
    }

    pub fn as_context(&self) -> &JSContext {
        &self.inner
    }

    /// Explicitly releases this owned context before the end of its scope.
    pub fn release(self) {}
}

impl std::ops::Deref for JSGlobalContext {
    type Target = JSContext;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for JSGlobalContext {
    fn drop(&mut self) {
        unsafe {
            let data_ptr = JSContextGetSharedData(self.inner.inner);
            if PrivateDataWrapper::drop_erased(data_ptr).is_dropped() {
                JSContextSetSharedData(self.inner.inner, std::ptr::null_mut());
            }
            JSInspectorDisconnect(self.inner.inner);
            JSGlobalContextRelease(self.inner.inner);
        }
    }
}

impl Default for JSGlobalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for JSGlobalContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSGlobalContext").finish()
    }
}

impl<T: 'static> TypedJSContext<T> {
    /// Creates a new `TypedJSContext` with the given shared data.
    pub fn new(data: T) -> Self {
        let inner = JSGlobalContext::new();
        inner.set_shared_data(data).unwrap();
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new `TypedJSContext` with the given class and shared data.
    pub fn new_with(class: &JSClass, data: T) -> Self {
        let inner = JSGlobalContext::new_with(class);
        inner.set_shared_data(data).unwrap();
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new `TypedJSContext` from an existing `JSContextGroup` with shared data.
    pub fn new_in_group(group: &JSContextGroup, data: T) -> Self {
        let inner = group.new_context();
        inner.set_shared_data(data).unwrap();
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new `TypedJSContext` from an existing `JSContextGroup`, with the given class and shared data.
    pub fn new_in_group_with_class(
        group: &JSContextGroup,
        class: &JSClass,
        data: T,
    ) -> Self {
        let inner = group.new_context_with_class(class);
        inner.set_shared_data(data).unwrap();
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Gets an immutable reference to the shared data.
    pub fn get_data(&self) -> Option<&T> {
        self.inner.get_shared_data::<T>()
    }

    /// Gets a mutable reference to the shared data.
    ///
    /// # Safety
    /// The caller must ensure that no other references to the shared data exist.
    pub unsafe fn get_data_mut(&self) -> Option<&mut T> {
        self.inner.get_shared_data_mut::<T>()
    }

    /// Takes ownership of the shared data, removing it from the context.
    /// Returns `None` if no data is set.
    ///
    /// # Safety
    /// The caller must ensure that no other references to the shared data exist.
    pub unsafe fn take_data(&self) -> PrivateDataTakeResult<T> {
        self.inner.take_shared_data::<T>()
    }

    /// Replaces the typed shared data.
    ///
    /// This requires `&mut self`, so safe callers cannot hold references
    /// returned by [`get_data`] while replacing the value.
    pub fn replace_data(&mut self, data: T) -> PrivateDataSetStatus {
        unsafe { self.inner.replace_shared_data(data) }
    }
}

impl<T: 'static> std::ops::Deref for TypedJSContext<T> {
    type Target = JSContext;

    fn deref(&self) -> &Self::Target {
        self.inner.as_context()
    }
}

impl<T: 'static> Drop for TypedJSContext<T> {
    fn drop(&mut self) {
        unsafe {
            // Automatically drop the shared data
            self.inner.drop_shared_data::<T>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{self as rust_jsc};

    use rust_jsc_macros::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    static BORROWED_CONTEXT_CALLBACK_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropCounter {
        drops: Arc<AtomicUsize>,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

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
        assert_eq!(format!("{:?}", ctx), "JSGlobalContext");
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
        assert_eq!(format!("{:?}", group), "OwnedJSContextGroup");
    }

    #[test]
    fn test_js_context_with_group() {
        let group = JSContextGroup::new();
        let ctx = group.new_context();
        assert_eq!(format!("{:?}", ctx), "JSGlobalContext");
    }

    #[test]
    fn test_owned_global_context_retain_survives_original_handle_drop() {
        let retained: JSGlobalContext = {
            let ctx: JSGlobalContext = JSContext::new();
            let retained = ctx.retain();
            let result = ctx.evaluate_script("20 + 1", None).unwrap();
            assert_eq!(result.as_number().unwrap(), 21.0);
            retained
        };

        let result = retained.evaluate_script("40 + 2", None).unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_borrowed_callback_context_does_not_release_owned_context() {
        BORROWED_CONTEXT_CALLBACK_COUNT.store(0, Ordering::SeqCst);

        #[callback]
        fn borrowed_context_callback(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
        ) -> JSResult<JSValue> {
            BORROWED_CONTEXT_CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
            ctx.evaluate_script("globalThis.borrowedContextTouched = true", None)?;
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let function = rust_jsc::JSFunction::callback(
            &ctx,
            Some("borrowedContextCallback"),
            Some(borrowed_context_callback),
        );
        ctx.global_object()
            .set_property("borrowedContextCallback", &function, Default::default())
            .unwrap();

        ctx.evaluate_script("borrowedContextCallback()", None)
            .unwrap();
        assert_eq!(BORROWED_CONTEXT_CALLBACK_COUNT.load(Ordering::SeqCst), 1);

        let result = ctx
            .evaluate_script("globalThis.borrowedContextTouched", None)
            .unwrap();
        assert!(result.as_boolean());

        let result = ctx.evaluate_script("6 * 7", None).unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_context_group_retain_survives_original_group_drop() {
        let retained: OwnedJSContextGroup = {
            let group: OwnedJSContextGroup = JSContextGroup::new();
            group.retain()
        };

        let ctx = retained.new_context();
        let result = ctx.evaluate_script("7 * 6", None).unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_context_group_from_context_is_borrowed() {
        let ctx = JSContext::new();
        let borrowed_group: JSContextGroup = ctx.group();
        let retained_group: OwnedJSContextGroup = borrowed_group.retain();

        let sibling = retained_group.new_context();
        let result = sibling.evaluate_script("21 + 21", None).unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);

        let direct = borrowed_group.new_context();
        let result = direct.evaluate_script("'borrowed-group'", None).unwrap();
        assert_eq!(result.as_string().unwrap().to_string(), "borrowed-group");
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
    fn test_js_context_evaluate_script_primitive_exception() {
        let ctx = JSContext::new();
        let error = ctx
            .evaluate_script("throw 'plain failure'", None)
            .unwrap_err();

        assert_eq!(error.name().unwrap().to_string(), "Error");
        assert_eq!(error.message().unwrap().to_string(), "plain failure");
    }

    #[test]
    fn test_js_context_evaluate_module_source() {
        let ctx = JSContext::new();
        let script = "console.log('Hello, world!'); 'kedojs'";
        let result = ctx.evaluate_module_from_source(script, "source.js", None);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Inspector / Debugger Tests
    // =========================================================================

    #[test]
    fn test_inspector_set_inspectable() {
        let ctx = JSContext::new();

        // Initially not inspectable
        ctx.set_inspectable(false);
        assert!(!ctx.is_inspectable());

        // Enable inspectable
        ctx.set_inspectable(true);
        assert!(ctx.is_inspectable());

        // Disable inspectable
        ctx.set_inspectable(false);
        assert!(!ctx.is_inspectable());
    }

    #[test]
    fn test_inspector_connect_disconnect() {
        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        #[inspector_callback]
        fn callback(_message: &str) {}

        // Connect inspector
        ctx.set_inspector_callback(Some(callback));
        assert!(ctx.inspector_is_connected());

        // Disconnect inspector
        ctx.inspector_disconnect();
        assert!(!ctx.inspector_is_connected());
    }

    #[test]
    fn test_inspector_send_message() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static RECEIVED: AtomicBool = AtomicBool::new(false);

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        #[inspector_callback]
        fn callback(message: &str) {
            // Verify we receive a response
            if message.contains("\"id\":1") {
                RECEIVED.store(true, Ordering::SeqCst);
            }
        }

        ctx.set_inspector_callback(Some(callback));

        // Send a simple Runtime.evaluate command
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#,
        );

        assert!(RECEIVED.load(Ordering::SeqCst), "Should receive response");
        ctx.inspector_disconnect();
    }

    #[test]
    fn test_inspector_debugger_enable_disable() {
        use std::sync::atomic::{AtomicU32, Ordering};

        static RESPONSE_COUNT: AtomicU32 = AtomicU32::new(0);

        let ctx = JSContext::new();
        ctx.set_inspectable(true);

        #[inspector_callback]
        fn callback(message: &str) {
            if message.contains("\"result\"") {
                RESPONSE_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        ctx.set_inspector_callback(Some(callback));

        // Enable debugger
        ctx.inspector_send_message(
            r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#,
        );

        // Disable debugger
        ctx.inspector_send_message(
            r#"{"id": 2, "method": "Debugger.disable", "params": {}}"#,
        );

        assert!(
            RESPONSE_COUNT.load(Ordering::SeqCst) >= 2,
            "Should receive responses for enable and disable"
        );

        ctx.inspector_disconnect();
    }

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

    #[test]
    fn test_shared_data_type_safe() {
        let ctx = JSContext::new();
        assert_eq!(ctx.set_shared_data(42i32), PrivateDataSetStatus::Set);
        let data = ctx.get_shared_data::<i32>().unwrap();
        assert_eq!(*data, 42);
    }

    #[test]
    fn test_shared_data_wrong_type_returns_none() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("hello"));
        // Requesting the wrong type must return None, not UB
        assert!(ctx.get_shared_data::<u64>().is_none());
        assert!(ctx.get_shared_data::<i32>().is_none());
        assert!(ctx.get_shared_data::<Vec<u8>>().is_none());
        // Correct type works
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "hello");
    }

    #[test]
    fn test_shared_data_multiple_reads() {
        let ctx = JSContext::new();
        ctx.set_shared_data(100u64);
        // Multiple reads should all succeed (no double-free)
        assert_eq!(*ctx.get_shared_data::<u64>().unwrap(), 100);
        assert_eq!(*ctx.get_shared_data::<u64>().unwrap(), 100);
        assert_eq!(*ctx.get_shared_data::<u64>().unwrap(), 100);
    }

    #[test]
    fn test_shared_data_mut() {
        let ctx = JSContext::new();
        ctx.set_shared_data(10i32);
        if let Some(data) = unsafe { ctx.get_shared_data_mut::<i32>() } {
            *data = 20;
        }
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 20);
    }

    #[test]
    fn test_take_shared_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("take me"));
        let taken = unsafe { ctx.take_shared_data::<String>() }.unwrap();
        assert_eq!(taken, "take me");
        // After take, data is gone
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_shared_data_none_when_empty() {
        let ctx = JSContext::new();
        assert!(ctx.get_shared_data::<i32>().is_none());
        assert!(unsafe { ctx.take_shared_data::<i32>() }.is_none());
    }

    #[test]
    fn test_shared_data_refcell_safe_mutation() {
        use std::cell::RefCell;

        let ctx = JSContext::new();
        ctx.set_shared_data(RefCell::new(String::from("original")));

        // Safe interior mutability via RefCell
        let cell = ctx.get_shared_data::<RefCell<String>>().unwrap();
        *cell.borrow_mut() = String::from("mutated");

        // Re-read through get_shared_data — no unsafe needed
        let cell = ctx.get_shared_data::<RefCell<String>>().unwrap();
        assert_eq!(*cell.borrow(), "mutated");
    }

    #[test]
    fn test_shared_data_struct() {
        #[derive(Debug, PartialEq)]
        struct AppState {
            counter: u32,
            name: String,
        }

        let ctx = JSContext::new();
        ctx.set_shared_data(AppState {
            counter: 0,
            name: "test".to_string(),
        });

        let state = ctx.get_shared_data::<AppState>().unwrap();
        assert_eq!(state.counter, 0);
        assert_eq!(state.name, "test");

        // Wrong type returns None
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_shared_data_mut_wrong_type_returns_none() {
        let ctx = JSContext::new();
        ctx.set_shared_data(42i32);

        // SAFETY: no other references exist
        let result = unsafe { ctx.get_shared_data_mut::<String>() };
        assert!(result.is_none());

        // Original data is still intact
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_take_shared_data_wrong_type_preserves_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("preserve me"));

        // Taking with wrong type should return None and NOT destroy the data
        assert_eq!(
            unsafe { ctx.take_shared_data::<i32>() },
            PrivateDataTakeResult::TypeMismatch
        );
        assert_eq!(
            unsafe { ctx.take_shared_data::<Vec<u8>>() },
            PrivateDataTakeResult::TypeMismatch
        );

        // Data should still be accessible with correct type
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "preserve me");

        // Now take with correct type
        let taken = unsafe { ctx.take_shared_data::<String>() }.unwrap();
        assert_eq!(taken, "preserve me");
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_drop_shared_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("drop me"));

        assert!(ctx.get_shared_data::<String>().is_some());
        assert_eq!(
            unsafe { ctx.drop_shared_data::<String>() },
            PrivateDataDropStatus::Dropped
        );
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_drop_shared_data_wrong_type_preserves_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("keep me"));

        assert_eq!(
            unsafe { ctx.drop_shared_data::<i32>() },
            PrivateDataDropStatus::TypeMismatch
        );
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "keep me");

        assert_eq!(
            unsafe { ctx.drop_shared_data::<String>() },
            PrivateDataDropStatus::Dropped
        );
        assert_eq!(
            unsafe { ctx.drop_shared_data::<String>() },
            PrivateDataDropStatus::Empty
        );
    }

    #[test]
    fn test_set_shared_data_does_not_replace_existing_data() {
        let ctx = JSContext::new();
        assert_eq!(ctx.set_shared_data(42i32), PrivateDataSetStatus::Set);
        assert_eq!(
            ctx.set_shared_data(String::from("new type")),
            PrivateDataSetStatus::AlreadySet
        );

        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 42);
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_replace_shared_data_drops_existing_data() {
        let drops = Arc::new(AtomicUsize::new(0));
        let ctx = JSContext::new();

        assert_eq!(
            ctx.set_shared_data(DropCounter {
                drops: drops.clone()
            }),
            PrivateDataSetStatus::Set
        );
        assert_eq!(drops.load(Ordering::SeqCst), 0);

        assert_eq!(
            unsafe {
                ctx.replace_shared_data(DropCounter {
                    drops: drops.clone(),
                })
            },
            PrivateDataSetStatus::Replaced
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_owned_global_context_drops_shared_data() {
        let drops = Arc::new(AtomicUsize::new(0));

        {
            let ctx = JSContext::new();
            assert_eq!(
                ctx.set_shared_data(DropCounter {
                    drops: drops.clone()
                }),
                PrivateDataSetStatus::Set
            );
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }

        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_typed_context_replace_data_drops_once() {
        let drops = Arc::new(AtomicUsize::new(0));

        {
            let mut ctx = TypedJSContext::new(DropCounter {
                drops: drops.clone(),
            });
            assert_eq!(drops.load(Ordering::SeqCst), 0);

            assert_eq!(
                ctx.replace_data(DropCounter {
                    drops: drops.clone()
                }),
                PrivateDataSetStatus::Replaced
            );
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }

        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_shared_data_replace() {
        let ctx = JSContext::new();
        ctx.set_shared_data(42i32);
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 42);

        // Take old data, then set new data of a different type
        let old = unsafe { ctx.take_shared_data::<i32>() }.unwrap();
        assert_eq!(old, 42);

        ctx.set_shared_data(String::from("new type"));
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "new type");
        // Old type is gone
        assert!(ctx.get_shared_data::<i32>().is_none());
    }

    #[test]
    fn test_shared_data_vec() {
        let ctx = JSContext::new();
        ctx.set_shared_data(vec![1u8, 2, 3, 4, 5]);

        let data = ctx.get_shared_data::<Vec<u8>>().unwrap();
        assert_eq!(data, &[1, 2, 3, 4, 5]);

        // Mutate via unsafe
        // SAFETY: no other references exist
        let data = unsafe { ctx.get_shared_data_mut::<Vec<u8>>() }.unwrap();
        data.push(6);

        let data = ctx.get_shared_data::<Vec<u8>>().unwrap();
        assert_eq!(data, &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_shared_data_zero_sized_type() {
        let ctx = JSContext::new();
        ctx.set_shared_data(());

        assert!(ctx.get_shared_data::<()>().is_some());
        assert!(ctx.get_shared_data::<i32>().is_none());

        assert!(matches!(
            unsafe { ctx.take_shared_data::<()>() },
            PrivateDataTakeResult::Taken(())
        ));
    }

    #[test]
    fn test_shared_data_read_after_mut() {
        let ctx = JSContext::new();
        ctx.set_shared_data(100i32);

        // Mutate
        // SAFETY: no other references exist
        {
            let data = unsafe { ctx.get_shared_data_mut::<i32>() }.unwrap();
            *data = 200;
        }
        // Reference dropped, now safe to read
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 200);
    }

    #[test]
    fn test_shared_data_uaf_scenario() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("alive"));

        // Create a borrowed alias to simulate another callback/context view.
        let ctx_alias = *ctx.as_context();

        // 1. Get reference from first context
        // The reference lifetime is tied to `ctx`
        let data_ref = ctx.get_shared_data::<String>().unwrap();
        assert_eq!(data_ref, "alive");

        // 2. Take ownership from alias (simulating concurrent access or passed handle)
        // This frees the memory while data_ref is still active in Rust's eyes
        // (because the compiler doesn't know ctx and ctx_alias share state)
        let taken = unsafe { ctx_alias.take_shared_data::<String>() };
        assert_eq!(taken.unwrap(), "alive");

        // 3. data_ref is now dangling!
        // Accessing it here would be UAF and likely crash or print garbage.
        // println!("Dangling ref: {}", data_ref); // THIS CAUSES UB/CRASH

        // Verify the data is logically gone from the context
        assert!(ctx.get_shared_data::<String>().is_none());

        // Borrowed contexts are Copy views and never release the global context.
        let _ = ctx_alias;
    }

    #[test]
    fn test_typed_js_context() {
        let typed_ctx = TypedJSContext::new(String::from("typed data"));

        // Deref allows calling JSContext methods
        let global = typed_ctx.global_object();
        assert!(global.is_object());

        // Typed access to data
        let data = typed_ctx.get_data().unwrap();
        assert_eq!(data, "typed data");

        // Take data
        let taken = unsafe { typed_ctx.take_data() }.unwrap();
        assert_eq!(taken, "typed data");
        assert!(typed_ctx.get_data().is_none());
    }
}
