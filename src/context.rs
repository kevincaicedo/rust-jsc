use rust_jsc_sys::{
    JSCheckScriptSyntax, JSContextGetGlobalContext, JSContextGetGlobalObject,
    JSContextGetGroup, JSContextGetSharedData, JSContextGroupCreate,
    JSContextGroupRelease, JSContextGroupRetain, JSContextRef, JSContextSetSharedData,
    JSEvaluateScript, JSGarbageCollect, JSGlobalContextCopyName,
    JSGlobalContextCreateInGroup, JSGlobalContextIsInspectable, JSGlobalContextRelease,
    JSGlobalContextRetain, JSGlobalContextSetInspectable, JSGlobalContextSetName,
    JSLoadAndEvaluateModule, JSValueRef,
};

use crate::{JSContext, JSContextGroup, JSObject, JSResult, JSString, JSValue};

impl JSContextGroup {
    fn from(context: JSContextRef) -> Self {
        let global_context = unsafe { JSContextGetGlobalContext(context) };
        unsafe {
            JSGlobalContextRetain(global_context);
        }
        let context_group = unsafe { JSContextGetGroup(global_context) };
        unsafe {
            JSContextGroupRetain(context_group);
        }
        Self {
            context_group,
            global_context,
        }
    }

    /// Creates a new `JSContextGroup` object.
    fn new() -> Self {
        let context_group = unsafe { JSContextGroupCreate() };
        let global_context =
            unsafe { JSGlobalContextCreateInGroup(context_group, std::ptr::null_mut()) };
        Self {
            context_group,
            global_context,
        }
    }
}

impl Drop for JSContextGroup {
    fn drop(&mut self) {
        unsafe {
            JSGlobalContextRelease(self.global_context);
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
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ```
    pub fn new() -> Self {
        let group = JSContextGroup::new();
        let global_context = group.global_context;
        let ctx = unsafe { JSContextGetGlobalContext(global_context) };
        Self { inner: ctx, group }
    }

    /// Garbage collects the JavaScript execution context.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.garbage_collect();
    /// ```
    pub fn garbage_collect(&self) {
        unsafe { JSGarbageCollect(self.inner) }
    }

    /// Checks the syntax of a JavaScript script.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.check_syntax("console.log('Hello, world!');", 1);
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

    /// Creates a new `JSContext` object with a given group.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::{JSContext, JSContextGroup};
    ///
    /// let group = JSContextGroup::new();
    /// let ctx = JSContext::with_group(group);
    /// ```
    pub fn with_group(group: JSContextGroup) -> Self {
        let ctx = unsafe { JSContextGetGlobalContext(group.global_context) };
        Self { inner: ctx, group }
    }

    /// Gets the global object of the JavaScript execution context.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let global_object = ctx.global_object();
    /// ```
    pub fn global_object(&self) -> JSObject {
        JSObject::from_ref(unsafe { JSContextGetGlobalObject(self.inner) }, self.inner)
    }

    /// Evaluates a JavaScript module.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let filename = "/path/filename.js");
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
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let result = ctx.evaluate_script("console.log('Hello, world!'); 'kedojs'", 0);
    /// assert!(result.is_ok());
    /// ```
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
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let is_inspectable = ctx.is_inspectable();
    /// ```
    ///
    pub fn is_inspectable(&self) -> bool {
        unsafe { JSGlobalContextIsInspectable(self.group.global_context) }
    }

    /// Sets whether a context is inspectable.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_inspectable(true);
    /// ```
    pub fn set_inspectable(&self, inspectable: bool) {
        unsafe { JSGlobalContextSetInspectable(self.group.global_context, inspectable) };
    }

    /// Sets the name exposed when inspecting a context.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_name("KedoJS");
    /// ```
    pub fn set_name(&self, name: &str) {
        let name: JSString = name.into();
        unsafe { JSGlobalContextSetName(self.group.global_context, name.inner) }
    }

    /// Gets a copy of the name of a context.
    ///
    /// e.g.
    /// ```
    /// use rust_jsc::context::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// let name = ctx.get_name();
    /// ```
    ///
    /// # Returns
    ///
    /// Returns a `JSString` object.
    pub fn get_name(&self) -> JSString {
        let name = unsafe { JSGlobalContextCopyName(self.group.global_context) };
        name.into()
    }

    pub(crate) fn set_shared_data<T>(ctx_inner: JSContextRef, data: Box<T>) {
        let data_ptr = Box::into_raw(data);
        unsafe { JSContextSetSharedData(ctx_inner, data_ptr as _) }
    }

    pub(crate) fn get_shared_data<T>(ctx_inner: JSContextRef) -> Option<Box<T>> {
        let data_ptr = unsafe { JSContextGetSharedData(ctx_inner) };

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
    fn from(ctx: JSContextRef) -> Self {
        let group = JSContextGroup::from(ctx);
        Self { inner: ctx, group }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_context() {
        let ctx = JSContext::new();
        assert_eq!(format!("{:?}", ctx), "JSContext");
    }

    #[test]
    fn test_js_context_group() {
        let group = JSContextGroup::new();
        assert_eq!(format!("{:?}", group), "JSContextGroup");
    }

    #[test]
    fn test_js_context_with_group() {
        let group = JSContextGroup::new();
        let ctx = JSContext::with_group(group);
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
        let result = ctx.check_syntax("console.log('Hello, world!');", 1);
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
        let result = ctx.evaluate_script("console.log('Hello, world!'); 'kedojs'", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shared_data() {
        let ctx = JSContext::new();
        let data = Box::new(10);
        JSContext::set_shared_data(ctx.inner, data);

        let shared_data = JSContext::get_shared_data::<i32>(ctx.inner).unwrap();
        assert_eq!(*shared_data, 10);
    }

    #[test]
    fn test_shared_data_null() {
        let ctx = JSContext::new();
        let shared_data = JSContext::get_shared_data::<i32>(ctx.inner);
        assert!(shared_data.is_none());
    }
}
