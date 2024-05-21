use rust_jsc_sys::{
    JSCheckScriptSyntax, JSContextGetGlobalContext, JSContextGetGlobalObject,
    JSContextGetGroup, JSContextGetSharedData, JSContextGroupCreate, JSContextGroupRef,
    JSContextGroupRelease, JSContextRef, JSContextSetSharedData, JSEvaluateScript,
    JSGarbageCollect, JSGetMemoryUsageStatistics, JSGlobalContextCopyName,
    JSGlobalContextCreate, JSGlobalContextCreateInGroup, JSGlobalContextIsInspectable,
    JSGlobalContextRef, JSGlobalContextRelease, JSGlobalContextRetain,
    JSGlobalContextSetInspectable, JSGlobalContextSetName,
    JSGlobalContextSetUnhandledRejectionCallback, JSLoadAndEvaluateModule, JSValueRef,
};

use crate::{JSClass, JSContext, JSContextGroup, JSObject, JSResult, JSString, JSValue};

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
    /// assert_eq!(is_inspectable, false);
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
    // use std::mem::ManuallyDrop;

    use super::*;

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
