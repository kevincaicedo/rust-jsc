use crate::{
    not_send_or_sync, JSClass, JSContext, JSContextGroup, JSError, JSGlobalContext,
    JSObject, JSResult, JSString, JSValue, NotSendOrSync, OwnedJSContextGroup,
    PrivateDataDropStatus, PrivateDataMut, PrivateDataRef, PrivateDataSetStatus,
    PrivateDataTakeResult, PrivateDataWrapper, ProtectedObject, TypedJSContext,
    UnhandledRejectionHandler,
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
    JSInspectorSetPauseEventCallback, JSModuleLinkAndEvaluate, JSModuleLoad,
    JSModuleLoadAndEvaluate, JSModuleLoadAndEvaluateFromSource, JSModuleLoadFromSource,
    JSModuleLoaderSetCallbacks, JSRunDeferredWork, JSRunMicrotasks,
    JSSyntheticModuleCreate, JSUncaughtExceptionAtEventLoop, JSUncaughtExceptionHandler,
    JSValueRef,
};
use std::{error::Error, ffi::CString, fmt};

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

    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> OwnedJSContextGroup {
        OwnedJSContextGroup::new()
    }

    pub fn retain(&self) -> OwnedJSContextGroup {
        // SAFETY: `self.context_group` is a live borrowed context-group handle;
        // retaining returns an owned reference that must be released by RAII.
        let context_group = unsafe { JSContextGroupRetain(self.context_group) };
        OwnedJSContextGroup {
            inner: Self::from_ref(context_group),
        }
    }

    pub fn new_context(&self) -> JSGlobalContext {
        // SAFETY: `self.context_group` is live and null selects the default
        // global object class. JavaScriptCore returns an owned global context.
        let ctx = unsafe {
            JSGlobalContextCreateInGroup(self.context_group, std::ptr::null_mut())
        };
        JSGlobalContext::from_owned_ref(ctx)
    }

    pub fn new_context_with_class(&self, class: &JSClass) -> JSGlobalContext {
        let ctx =
            // SAFETY: `self.context_group` and `class.inner` are live handles.
            // JavaScriptCore returns an owned global context.
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
        // SAFETY: JavaScriptCore creates and returns an owned context-group
        // reference for the current thread.
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
        // SAFETY: `self.inner.context_group` is the owned reference retained or
        // created for this RAII wrapper and is released exactly once here.
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

/// Error returned when an outbound inspector protocol message cannot cross the
/// JavaScriptCore C API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorMessageError {
    InteriorNul { position: usize },
}

impl fmt::Display for InspectorMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul { position } => {
                write!(
                    formatter,
                    "inspector protocol message contains an interior NUL byte at offset {position}"
                )
            }
        }
    }
}

impl Error for InspectorMessageError {}

/// Borrowed inspector protocol message received from JavaScriptCore.
///
/// The borrowed text is valid only for the callback duration unless the
/// embedder copies it into [`OwnedInspectorMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorInboundMessage<'message> {
    message: &'message str,
}

/// Borrowed inspector protocol message that has been validated for sending to
/// JavaScriptCore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorOutboundMessage<'message> {
    message: &'message str,
}

/// Owned inspector protocol message for queues, debugger bridges, and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedInspectorMessage {
    message: String,
}

impl<'message> InspectorInboundMessage<'message> {
    pub const fn borrowed(message: &'message str) -> Self {
        Self { message }
    }

    pub const fn as_str(&self) -> &'message str {
        self.message
    }

    pub fn to_owned_message(&self) -> OwnedInspectorMessage {
        OwnedInspectorMessage {
            message: self.message.to_owned(),
        }
    }
}

impl<'message> InspectorOutboundMessage<'message> {
    /// Validates a borrowed inspector protocol message for sending.
    ///
    /// # Errors
    /// Returns [`InspectorMessageError::InteriorNul`] when the message contains
    /// a NUL byte that cannot cross the JavaScriptCore C API.
    pub fn borrowed(message: &'message str) -> Result<Self, InspectorMessageError> {
        if let Some(position) = message.as_bytes().iter().position(|byte| *byte == 0) {
            return Err(InspectorMessageError::InteriorNul { position });
        }

        Ok(Self { message })
    }

    pub const fn as_str(&self) -> &'message str {
        self.message
    }
}

impl OwnedInspectorMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }

    pub fn as_inbound(&self) -> InspectorInboundMessage<'_> {
        InspectorInboundMessage::borrowed(&self.message)
    }

    /// Borrows this message as a validated outbound protocol message.
    ///
    /// # Errors
    /// Returns [`InspectorMessageError::InteriorNul`] when the message contains
    /// a NUL byte that cannot cross the JavaScriptCore C API.
    pub fn as_outbound(
        &self,
    ) -> Result<InspectorOutboundMessage<'_>, InspectorMessageError> {
        InspectorOutboundMessage::borrowed(&self.message)
    }
}

/// Raw inspector protocol callback accepted by [`InspectorSessionBuilder`].
///
/// Use `#[inspector_callback]` to generate this function pointer from a Rust
/// function that receives `&str`.
pub type InspectorMessageHandler =
    unsafe extern "C" fn(*const std::os::raw::c_char, usize);

/// Raw inspector pause-loop callback accepted by [`InspectorSessionBuilder`].
///
/// Use `#[inspector_pause_event_callback]` to generate this function pointer
/// from a Rust function that receives [`JSContext`] and [`InspectorPauseEvent`].
pub type InspectorPauseEventHandler =
    unsafe extern "C" fn(JSContextRef, rust_jsc_sys::InspectorPauseEvent);

/// Builder for a direct JavaScriptCore inspector frontend session.
///
/// The builder keeps the safe path explicit: provide a message callback, opt
/// into a pause-loop callback only when a debugger pump is needed, then call
/// [`connect`](Self::connect). The resulting [`InspectorSession`] disconnects
/// and clears callbacks on drop.
#[must_use]
pub struct InspectorSessionBuilder<'ctx> {
    context: &'ctx JSContext,
    message_callback: Option<InspectorMessageHandler>,
    pause_event_callback: Option<InspectorPauseEventHandler>,
    inspectable: bool,
}

/// RAII handle for a direct JavaScriptCore inspector frontend connection.
///
/// The session borrows the context it is attached to, sends protocol messages
/// through a fallible API, and disconnects the frontend when dropped. It is
/// context-affine and intentionally not `Send` or `Sync`.
#[must_use]
pub struct InspectorSession<'ctx> {
    context: &'ctx JSContext,
    connected: bool,
    _not_send_or_sync: NotSendOrSync,
}

impl<'ctx> InspectorSessionBuilder<'ctx> {
    pub(crate) fn new(context: &'ctx JSContext) -> Self {
        Self {
            context,
            message_callback: None,
            pause_event_callback: None,
            inspectable: true,
        }
    }

    /// Registers the protocol message callback for this session.
    ///
    /// The generated callback receives a borrowed UTF-8 message for the
    /// duration of the callback only. Copy the message before returning if it
    /// must be stored.
    pub fn on_message(mut self, callback: InspectorMessageHandler) -> Self {
        self.message_callback = Some(callback);
        self
    }

    /// Registers a debugger pause-loop pump callback.
    ///
    /// Pause callbacks run on the JavaScriptCore thread while execution is
    /// paused. Keep the callback short; use it to queue work or send simple
    /// inspector protocol messages, not to tear down the VM.
    pub fn on_pause_event(mut self, callback: InspectorPauseEventHandler) -> Self {
        self.pause_event_callback = Some(callback);
        self
    }

    /// Controls the context's inspectable flag before connecting.
    ///
    /// The default is `true`, which matches the direct-frontend debugger path.
    pub fn inspectable(mut self, inspectable: bool) -> Self {
        self.inspectable = inspectable;
        self
    }

    /// Connects the direct inspector frontend.
    ///
    /// # Errors
    /// Returns a JavaScript `Error` if no message callback was provided or if
    /// this context already has an active inspector connection.
    pub fn connect(self) -> JSResult<InspectorSession<'ctx>> {
        let Some(message_callback) = self.message_callback else {
            return Err(JSError::from_message(
                self.context,
                "inspector session requires a message callback",
            ));
        };

        if self.context.inspector_is_connected() {
            return Err(JSError::from_message(
                self.context,
                "context already has an active inspector session",
            ));
        }

        self.context.set_inspectable(self.inspectable);
        self.context.set_inspector_callback(Some(message_callback));
        self.context
            .set_inspector_pause_event_callback(self.pause_event_callback);

        Ok(InspectorSession {
            context: self.context,
            connected: true,
            _not_send_or_sync: not_send_or_sync(),
        })
    }
}

impl InspectorSession<'_> {
    /// Returns the context this inspector session is attached to.
    pub fn context(&self) -> &JSContext {
        self.context
    }

    /// Returns whether JavaScriptCore still reports the inspector as connected.
    pub fn is_connected(&self) -> bool {
        self.connected && self.context.inspector_is_connected()
    }

    /// Sends one JSON inspector protocol message to JavaScriptCore.
    ///
    /// # Errors
    /// Returns a JavaScript `Error` if the session is disconnected or if the
    /// message contains an interior NUL byte that cannot cross the C API.
    pub fn send_message(&self, message: &str) -> JSResult<()> {
        if !self.is_connected() {
            return Err(JSError::from_message(
                self.context,
                "inspector session is disconnected",
            ));
        }

        self.context.inspector_send_message(message)
    }

    /// Sends a validated inspector protocol message to JavaScriptCore.
    ///
    /// Use [`InspectorOutboundMessage::borrowed`] or
    /// [`OwnedInspectorMessage::as_outbound`] when the message is produced by a
    /// debugger bridge before it reaches the session.
    ///
    /// # Errors
    /// Returns a JavaScript `Error` if the session is disconnected.
    pub fn send_protocol_message(
        &self,
        message: InspectorOutboundMessage<'_>,
    ) -> JSResult<()> {
        self.send_message(message.as_str())
    }

    /// Disconnects this session before drop.
    pub fn disconnect(mut self) {
        self.disconnect_inner();
    }

    fn disconnect_inner(&mut self) {
        if self.connected {
            self.context.inspector_disconnect();
            self.connected = false;
        }
    }
}

impl Drop for InspectorSession<'_> {
    fn drop(&mut self) {
        self.disconnect_inner();
    }
}

impl UnhandledRejectionHandler {
    /// Returns the protected JavaScript function installed as the handler.
    pub fn function(&self) -> &JSObject {
        self.callback.object()
    }
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
        // SAFETY: caller supplies a live context pointer; JavaScriptCore returns
        // the associated global context without transferring ownership.
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
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> JSGlobalContext {
        JSGlobalContext::new()
    }

    pub fn new_with(class: &JSClass) -> JSGlobalContext {
        JSGlobalContext::new_with(class)
    }

    pub fn retain(&self) -> JSGlobalContext {
        // SAFETY: `self.inner` is a live global context; retaining returns an
        // owned reference released by `JSGlobalContext`.
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
        // SAFETY: `self.inner` is a live global context on the current thread.
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
        // SAFETY: `self.inner` is a live global context. JavaScriptCore returns a
        // borrowed object handle associated with the same context.
        let result = unsafe { JSGetMemoryUsageStatistics(self.inner) };
        JSObject::from_ref(result, self.inner)
    }

    /// Sets a callback function that is called when a promise is rejected and no handler is provided.
    /// The callback is called with the rejected promise and the reason for the rejection.
    ///
    /// Prefer [`set_unhandled_rejection_handler`](Self::set_unhandled_rejection_handler)
    /// for new code. It validates context/callability and returns a guard that
    /// keeps the callback protected while the host stores it.
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
        // SAFETY: `self.inner` and `function.inner` are live handles.
        // JavaScriptCore stores the callback and initializes `exception` if the
        // function is invalid for this context.
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

    /// Installs a JavaScript function as the unhandled-rejection handler.
    ///
    /// This is the preferred safe path over
    /// [`set_unhandled_rejection_callback`](Self::set_unhandled_rejection_callback):
    /// it rejects cross-context objects and non-callable objects before calling
    /// JavaScriptCore, then returns an [`UnhandledRejectionHandler`] guard that
    /// keeps the function protected while the host stores the guard.
    ///
    /// Dropping the returned guard releases Rust's protection count but does
    /// not unregister JavaScriptCore's global handler. Replace the handler by
    /// installing another function, or drop the context during runtime
    /// shutdown.
    ///
    /// # Errors
    /// Returns a JavaScript `TypeError` when `function` belongs to another
    /// context, is not callable, or JavaScriptCore rejects the registration.
    pub fn set_unhandled_rejection_handler(
        &self,
        function: &JSObject,
    ) -> JSResult<UnhandledRejectionHandler> {
        if function.value.ctx != self.inner {
            return Err(JSError::new_typ(
                self,
                "unhandled rejection handler belongs to another context",
            )?);
        }

        if !function.is_function() {
            return Err(JSError::new_typ(
                self,
                "unhandled rejection handler must be callable",
            )?);
        }

        self.set_unhandled_rejection_callback(function.clone())?;

        Ok(UnhandledRejectionHandler {
            callback: ProtectedObject::new(function.clone()),
        })
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
        // SAFETY: `self.inner` is live. The handler pointer is either null or an
        // extern "C" function with JavaScriptCore's expected ABI.
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
        // SAFETY: `self.inner` is live. The callback pointer is either null or
        // an extern "C" function with JavaScriptCore's expected ABI.
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
        // SAFETY: `self.inner` and `script.inner` are live handles. Null source
        // URL is accepted by JavaScriptCore, and `exception` is checked below.
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
        // SAFETY: `self.inner` is live. JavaScriptCore returns a borrowed
        // context-group handle owned by the context.
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
        // SAFETY: `self.inner` is live. JavaScriptCore returns the context's
        // borrowed global object handle.
        JSObject::from_ref(unsafe { JSContextGetGlobalObject(self.inner) }, self.inner)
    }

    /// Starts loading, linking, and evaluating a JavaScript module.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rust_jsc::{module_loader, JSContext};
    ///
    /// let filename = "/path/filename.js";
    /// let ctx = JSContext::new();
    /// ctx.set_module_loader(module_loader::file_module_loader());
    /// let promise = ctx.evaluate_module(filename).unwrap();
    /// assert!(promise.is_object());
    /// ```
    ///
    /// The returned value is the JavaScript `Promise` created by
    /// JavaScriptCore. The host runtime decides when to pump microtasks.
    ///
    pub fn evaluate_module(&self, filename: &str) -> JSResult<JSValue> {
        let filename: JSString = filename.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.inner` and `filename.inner` are live handles.
        // JavaScriptCore initializes `exception` if module loading/evaluation
        // starts with a synchronous failure.
        let result = unsafe {
            JSModuleLoadAndEvaluate(self.inner, filename.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        if result.is_null() {
            return Err(JSError::new_typ(self, "failed to start module evaluation")?);
        }

        Ok(JSValue::new(result, self.inner))
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
    pub fn load_module(&self, key: &str) -> JSResult<JSValue> {
        let module_key: JSString = key.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result =
            // SAFETY: `self.inner` and `module_key.inner` are live handles.
            // JavaScriptCore initializes `exception` on synchronous failure.
            unsafe { JSModuleLoad(self.inner, module_key.inner, &mut exception) };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        if result.is_null() {
            return Err(JSError::new_typ(
                self,
                format!("failed to start loading module `{key}`"),
            )?);
        }

        Ok(JSValue::new(result, self.inner))
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
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.inner` and `module_key.inner` are live handles.
        // JavaScriptCore initializes `exception` on synchronous failure.
        let result = unsafe {
            JSModuleLinkAndEvaluate(self.inner, module_key.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

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
    ) -> JSResult<JSValue> {
        let source: JSString = source.into();
        let source_url: JSString = source_url.into();
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self.inner`, `source.inner`, and `source_url.inner` are live
        // handles for this call. JavaScriptCore initializes `exception` on
        // synchronous failure.
        let result = unsafe {
            JSModuleLoadFromSource(
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

        if result.is_null() {
            return Err(JSError::new_typ(
                self,
                "failed to start loading module source",
            )?);
        }

        Ok(JSValue::new(result, self.inner))
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
    ) -> JSResult<JSValue> {
        let source: JSString = source.into();
        let source_url: JSString = source_url.into();
        let mut exception: JSValueRef = std::ptr::null_mut();

        // SAFETY: `self.inner`, `source.inner`, and `source_url.inner` are live
        // handles for this call. JavaScriptCore initializes `exception` on
        // synchronous failure.
        let result = unsafe {
            JSModuleLoadAndEvaluateFromSource(
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

        if result.is_null() {
            return Err(JSError::new_typ(
                self,
                "failed to start module source evaluation",
            )?);
        }

        Ok(JSValue::new(result, self.inner))
    }

    /// Sets the module loader for a context.
    /// The module loader is used to load modules when evaluating a module.
    /// The module loader is called with the module key and the context. Resolver
    /// and fetch callbacks are required for source modules. The evaluate
    /// callback is only required for synthetic modules, and import.meta
    /// creation is optional.
    ///
    /// # Arguments
    /// - `module_loader`: A module loader wrapper, builder, or raw callback set.
    pub fn set_module_loader<L>(&self, module_loader: L)
    where
        L: Into<JSAPIModuleLoader>,
    {
        // SAFETY: `self.inner` is live, and the converted loader contains
        // JavaScriptCore-compatible callback pointers. Ownership of the callback
        // table follows the raw API contract.
        unsafe { JSModuleLoaderSetCallbacks(self.inner, module_loader.into()) };
    }

    /// Runs pending JavaScript microtasks for this context's VM.
    ///
    /// Module APIs return promises and do not drain microtasks internally.
    /// Embedders should call this only at explicit event-loop checkpoints.
    pub fn run_microtasks(&self) {
        // SAFETY: `self.inner` is a live global context. The host controls that
        // this is called from an event-loop checkpoint on the JSC thread.
        unsafe { JSRunMicrotasks(self.inner) };
    }

    /// Runs pending JavaScriptCore deferred work for this context's VM.
    ///
    /// WebAssembly module validation/compilation can complete through JSC's
    /// deferred-work timer before the corresponding module promise settles. Host
    /// runtimes should call this from an event-loop checkpoint, not from a JSC
    /// callback, resolver, fetcher, import-meta hook, inspector callback, or
    /// native function.
    pub fn run_deferred_work(&self) {
        // SAFETY: `self.inner` is a live global context. The host controls that
        // this is called from an event-loop checkpoint on the JSC thread.
        unsafe { JSRunDeferredWork(self.inner) };
    }

    /// Creates and registers a synthetic module with explicit exports.
    ///
    /// # Arguments
    /// - `key`: The resolved module key.
    /// - `exports`: Export name and value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_jsc::{JSContext, JSValue};
    ///
    /// let ctx = JSContext::new();
    /// let name = JSValue::string(&ctx, "rust-jsc");
    /// ctx.create_synthetic_module("@runtime/name", &[("default", &name)]).unwrap();
    /// ```
    pub fn create_synthetic_module(
        &self,
        key: &str,
        exports: &[(&str, &JSValue)],
    ) -> JSResult<JSValue> {
        if exports.iter().any(|(_, value)| value.ctx != self.inner) {
            return Err(JSError::new_typ(
                self,
                "synthetic module export value belongs to a different context",
            )?);
        }

        let key: JSString = key.into();
        let export_names: Vec<JSString> =
            exports.iter().map(|(name, _)| (*name).into()).collect();
        let export_name_refs: Vec<_> =
            export_names.iter().map(|name| name.inner).collect();
        let export_value_refs: Vec<_> =
            exports.iter().map(|(_, value)| value.inner).collect();
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: all export values were checked to belong to `self.inner`.
        // Export-name and value arrays remain alive for the duration of the
        // call, and JavaScriptCore initializes `exception` on failure.
        let result = unsafe {
            JSSyntheticModuleCreate(
                self.inner,
                key.inner,
                exports.len(),
                export_name_refs.as_ptr(),
                export_value_refs.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.inner);
            return Err(value.into());
        }

        if result.is_null() {
            return Err(JSError::new_typ(self, "failed to create synthetic module")?);
        }

        Ok(JSValue::new(result, self.inner))
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
        // SAFETY: `self.inner` and `script.inner` are live handles. Null
        // `this` and source URL are accepted by JavaScriptCore. `exception` is
        // checked before wrapping the result.
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

    /// Creates a builder for a direct JavaScriptCore inspector frontend.
    pub fn inspector_session(&self) -> InspectorSessionBuilder<'_> {
        InspectorSessionBuilder::new(self)
    }

    /// Sets the callback function for inspector messages.
    ///
    /// Prefer [`JSContext::inspector_session`] for new code. This low-level
    /// setter remains available for embedding layers that need to manage the
    /// connection manually.
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
    /// ctx.set_inspector_callback(Some(inspector_callback));
    /// ```
    pub fn set_inspector_callback(&self, callback: InspectorMessageCallback) {
        // SAFETY: `self.inner` is a live global context. The callback pointer is
        // either null or an extern "C" function with JavaScriptCore's expected
        // inspector-message ABI.
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
    /// ctx.inspector_send_message("{ method: \"Runtime.evaluate\", params: { expression: \"1 + 1\" } }")?;
    /// # Ok::<(), rust_jsc::JSError>(())
    /// ```
    ///
    /// # Errors
    /// Returns a JavaScript `Error` if `message` contains an interior NUL byte.
    pub fn inspector_send_message(&self, message: &str) -> JSResult<()> {
        let message = CString::new(message).map_err(|_| {
            JSError::from_message(
                self,
                "inspector protocol messages cannot contain interior NUL bytes",
            )
        })?;

        // SAFETY: `self.inner` is a live global context. `message` is a
        // NUL-terminated C string that remains valid for the duration of the C
        // call.
        unsafe {
            JSInspectorSendMessage(self.inner, message.as_ptr());
        }

        Ok(())
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
        // SAFETY: `self.inner` is a live global context. Null callback values
        // clear the direct frontend hooks before the context is reused or
        // dropped.
        unsafe {
            JSInspectorDisconnect(self.inner);
            JSInspectorSetPauseEventCallback(self.inner, None);
            JSInspectorSetCallback(self.inner, None);
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
        // SAFETY: `self.inner` is a live global context and the C API only
        // observes the inspector connection bit.
        unsafe { JSInspectorIsConnected(self.inner) }
    }

    /// Registers a pause-event callback.
    ///
    /// The callback is a low-level debugger pump notification. It runs on the
    /// JavaScriptCore thread while execution is inside the debugger pause loop.
    /// Keep it short and do not release the context, destroy the VM, or run
    /// arbitrary JavaScript from the callback.
    ///
    /// # Arguments
    /// - `callback`: A callback function.
    pub fn set_inspector_pause_event_callback(
        &self,
        callback: InspectorPauseEventCallback,
    ) {
        // SAFETY: `self.inner` is a live global context. The callback pointer is
        // either null or an extern "C" function with JavaScriptCore's expected
        // pause-event ABI.
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
        // SAFETY: `self.inner` is a live global context and the C API only
        // observes its inspectable flag.
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
        // SAFETY: `self.inner` is a live global context and this setter only
        // mutates JavaScriptCore's inspectable flag.
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
        // SAFETY: `self.inner` and `name.inner` are live handles. JavaScriptCore
        // copies/retains the name for the context.
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
        // SAFETY: `self.inner` is live and JavaScriptCore returns a retained
        // string reference for the context name.
        let name = unsafe { JSGlobalContextCopyName(self.inner) };
        // SAFETY: JavaScriptCore returned an owned JSStringRef for the context
        // name, and this wrapper releases it in Drop.
        unsafe { JSString::from_owned_ref(name) }
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
        // SAFETY: `self.inner` is live. JavaScriptCore stores but does not own
        // the opaque pointer, so Rust remains responsible for allocation/drop.
        let current = unsafe { JSContextGetSharedData(self.inner) };
        if !current.is_null() {
            return PrivateDataSetStatus::AlreadySet;
        }

        let ptr = PrivateDataWrapper::into_raw(data);
        // SAFETY: `ptr` was allocated by `PrivateDataWrapper::into_raw`; the
        // context slot was checked empty, so this does not overwrite live data.
        unsafe { JSContextSetSharedData(self.inner, ptr) };
        PrivateDataSetStatus::Set
    }

    /// Replaces shared data, dropping the existing Rust-owned allocation first.
    ///
    /// If the existing shared-data allocation has an active runtime borrow, the
    /// context is left unchanged and [`PrivateDataSetStatus::Borrowed`] is
    /// returned.
    pub fn replace_shared_data<T: 'static>(&self, data: T) -> PrivateDataSetStatus {
        // SAFETY: `self.inner` is live. JavaScriptCore stores but does not own
        // the opaque shared-data pointer.
        let current = unsafe { JSContextGetSharedData(self.inner) };
        // SAFETY: `current` is either null or a pointer previously stored by
        // these shared-data APIs; borrowed data must not be replaced.
        if unsafe { PrivateDataWrapper::is_borrowed(current) } {
            return PrivateDataSetStatus::Borrowed;
        }

        let status = if current.is_null() {
            PrivateDataSetStatus::Set
        } else {
            // SAFETY: `current` is the Rust-owned shared-data allocation
            // currently installed on this context, and no borrow is active.
            unsafe { PrivateDataWrapper::drop_erased(current) };
            // SAFETY: `self.inner` is live; clearing the slot prevents
            // JavaScriptCore from retaining a dangling Rust pointer.
            unsafe { JSContextSetSharedData(self.inner, std::ptr::null_mut()) };
            PrivateDataSetStatus::Replaced
        };

        let ptr = PrivateDataWrapper::into_raw(data);
        // SAFETY: `ptr` is a fresh Rust allocation and the old slot is either
        // empty or was cleared above.
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
    pub fn get_shared_data<T: 'static>(&self) -> Option<PrivateDataRef<'_, T>> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        // SAFETY: the pointer is either null or one installed by the safe
        // shared-data APIs; `borrow_ref` validates type and runtime borrow state.
        unsafe { PrivateDataWrapper::borrow_ref(data_ptr) }
    }

    /// Gets shared data for a context as an exclusive mutable guard.
    ///
    /// Returns `None` if no data is set or if `T` does not match the type
    /// that was originally stored with [`set_shared_data`]. It also returns
    /// `None` while any shared or mutable private-data borrow is active.
    ///
    /// # Interior mutability
    ///
    /// For local mutation that may be nested inside callbacks, storing a
    /// [`std::cell::RefCell<T>`] and using [`get_shared_data`] remains a useful
    /// pattern:
    ///
    /// ```no_run
    /// use rust_jsc::JSContext;
    /// use std::cell::RefCell;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(RefCell::new(10i32));
    ///
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
    /// if let Some(mut data) = ctx.get_shared_data_mut::<i32>() {
    ///     *data = 20;
    /// }
    /// assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 20);
    /// ```
    pub fn get_shared_data_mut<T: 'static>(&self) -> Option<PrivateDataMut<'_, T>> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        // SAFETY: the pointer is either null or one installed by the safe
        // shared-data APIs; `borrow_mut` validates type and exclusive borrow state.
        unsafe { PrivateDataWrapper::borrow_mut(data_ptr) }
    }

    /// Takes ownership of the shared data, removing it from the context.
    ///
    /// Returns `None` if no data is set or if `T` does not match the stored type.
    /// if T does not match, the data remains in place and is not freed,
    /// so it can still be retrieved with the correct type.
    ///
    /// If any shared or mutable private-data borrow is active, this method
    /// returns [`PrivateDataTakeResult::Borrowed`] and leaves the data in place.
    ///
    /// # Examples
    /// ```no_run
    /// use rust_jsc::JSContext;
    ///
    /// let ctx = JSContext::new();
    /// ctx.set_shared_data(42i32);
    /// let data: i32 = ctx.take_shared_data::<i32>().unwrap();
    /// assert_eq!(data, 42);
    /// assert!(ctx.get_shared_data::<i32>().is_none()); // data has been removed
    /// ```
    pub fn take_shared_data<T: 'static>(&self) -> PrivateDataTakeResult<T> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        // Only take ownership (and clear the JSC pointer) if the type matches.
        // On type mismatch the data stays in place — nothing is freed or lost.
        // SAFETY: the pointer is either null or one installed by the safe
        // shared-data APIs; `take` validates type and refuses active borrows.
        let result = unsafe { PrivateDataWrapper::take(data_ptr) };
        if result.is_taken() {
            // SAFETY: ownership was taken and the allocation is gone, so the
            // JavaScriptCore slot must be cleared.
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
    /// ctx.drop_shared_data::<String>(); // Clean up without taking ownership
    /// assert!(ctx.get_shared_data::<String>().is_none()); // data has been removed
    /// ```
    /// If the type does not match, this method returns
    /// [`PrivateDataDropStatus::TypeMismatch`] and leaves the data pointer
    /// intact. If any shared or mutable private-data borrow is active, this
    /// method returns [`PrivateDataDropStatus::Borrowed`] and leaves the data in
    /// place.
    pub fn drop_shared_data<T: 'static>(&self) -> PrivateDataDropStatus {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSContextGetSharedData(self.inner) };
        // SAFETY: the pointer is either null or one installed by the safe
        // shared-data APIs; `drop_raw` validates `T` and refuses active borrows.
        let status = unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
        if status.is_dropped() {
            // SAFETY: the allocation was dropped, so the JavaScriptCore slot
            // must be cleared to avoid a dangling pointer.
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
        // SAFETY: null selects JavaScriptCore's default global object class and
        // JavaScriptCore returns an owned global context reference.
        let ctx = unsafe { JSGlobalContextCreate(std::ptr::null_mut()) };
        Self::from_owned_ref(ctx)
    }

    /// Creates a new owned JavaScript global context with a custom global class.
    pub fn new_with(class: &JSClass) -> Self {
        // SAFETY: `class.inner` is a live JavaScriptCore class handle and
        // JavaScriptCore returns an owned global context reference.
        let ctx = unsafe { JSGlobalContextCreate(class.inner) };
        Self::from_owned_ref(ctx)
    }

    /// Retains a borrowed raw global context and returns an owned RAII handle.
    ///
    /// # Safety
    /// The raw pointer must be a valid `JSGlobalContextRef`.
    pub unsafe fn retain_from_raw(ctx: JSGlobalContextRef) -> Self {
        // SAFETY: caller guarantees `ctx` is a valid global context reference;
        // retaining returns an owned reference for the RAII wrapper.
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
        // SAFETY: `self.inner.inner` is the owned global context reference for
        // this RAII wrapper. Rust-owned shared data is dropped before the slot is
        // cleared, inspector callbacks are disconnected, and the global context
        // reference is released exactly once.
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

    /// Gets an immutable guard for the shared data.
    pub fn get_data(&self) -> Option<PrivateDataRef<'_, T>> {
        self.inner.get_shared_data::<T>()
    }

    /// Gets a mutable guard for the shared data.
    pub fn get_data_mut(&self) -> Option<PrivateDataMut<'_, T>> {
        self.inner.get_shared_data_mut::<T>()
    }

    /// Takes ownership of the shared data, removing it from the context.
    /// Returns `None` if no data is set.
    pub fn take_data(&self) -> PrivateDataTakeResult<T> {
        self.inner.take_shared_data::<T>()
    }

    /// Replaces the typed shared data.
    ///
    /// This requires `&mut self`, so safe callers cannot hold references
    /// returned by [`get_data`] while replacing the value.
    pub fn replace_data(&mut self, data: T) -> PrivateDataSetStatus {
        self.inner.replace_shared_data(data)
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
        // Automatically drop the shared data.
        self.inner.drop_shared_data::<T>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        self as rust_jsc, module_loader, module_loader::read_file_module_source,
        JSPromise, JSStringProtected,
    };

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
        key: JSValue,
        _referrer: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringProtected {
        let key = key.as_string().unwrap().to_string();
        if key == "@rust-jsc" {
            JSStringProtected::from("@rust-jsc")
        } else {
            JSStringProtected::from(key)
        }
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
    ) -> JSStringProtected {
        let key_value = key.as_string().unwrap();
        // resolve path to file system
        let test_module_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        // module key can start with ./ or ../
        let path = std::path::Path::new(test_module_dir).join(key_value.to_string());
        let module_path = std::fs::canonicalize(path).unwrap();

        JSStringProtected::from(module_path.to_str().unwrap())
    }

    #[module_fetch]
    fn module_loader_fetch(
        _ctx: JSContext,
        _key: JSValue,
        _attributes_value: JSValue,
        _script_fetcher: JSValue,
    ) -> JSStringProtected {
        // read file content
        let path_key = _key.as_string().unwrap().to_string();
        println!("Path key: {:?}", path_key);
        // check if the path is a file
        let Some(file_content) = read_file_module_source(std::path::Path::new(&path_key))
        else {
            unreachable!("Error reading file: {:?}", path_key);
        };

        JSStringProtected::from(file_content)
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
        ctx.set_module_loader(module_loader::file_module_loader());
        let promise = ctx.evaluate_module(filename).unwrap();
        assert!(promise.is_object());
        ctx.run_microtasks();
    }

    #[test]
    fn test_js_context_evaluate_module_fails() {
        let filename =
            concat!(env!("CARGO_MANIFEST_DIR"), "/non_exist/mock_path/wrong.js");
        let ctx = JSContext::new();
        ctx.set_module_loader(module_loader::file_module_loader());
        let error = ctx.evaluate_module(filename).unwrap_err();
        let message = error.message().unwrap().to_string();
        assert!(message.to_ascii_lowercase().contains("module"), "{message}");
    }

    #[test]
    fn test_file_module_loader_imports_json() {
        let test_dir = std::env::temp_dir()
            .join(format!("rust-jsc-json-loader-{}", std::process::id()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let json_path = test_dir.join("data.json");
        let module_path = test_dir.join("main.js");
        std::fs::write(&json_path, r#"{"name":"rust-jsc"}"#).unwrap();
        std::fs::write(
            &module_path,
            "import data from './data.json'; globalThis.jsonModuleName = data.name;",
        )
        .unwrap();

        let ctx = JSContext::new();
        ctx.set_module_loader(module_loader::file_module_loader());
        let promise = ctx
            .evaluate_module(module_path.to_string_lossy().as_ref())
            .unwrap();
        assert!(promise.is_object());
        ctx.run_microtasks();

        let name = ctx
            .evaluate_script("globalThis.jsonModuleName", None)
            .unwrap();
        assert_eq!(name.as_string().unwrap().to_string(), "rust-jsc");

        let _ = std::fs::remove_dir_all(test_dir);
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
        let promise = ctx
            .evaluate_module_from_source(script, "source.js", None)
            .unwrap();
        assert!(promise.is_object());
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

        #[inspector_callback]
        fn callback(_message: &str) {}

        let session = ctx
            .inspector_session()
            .on_message(callback)
            .connect()
            .unwrap();
        assert!(session.is_connected());

        drop(session);
        assert!(!ctx.inspector_is_connected());
    }

    #[test]
    fn test_inspector_session_requires_message_callback() {
        let ctx = JSContext::new();
        assert!(ctx.inspector_session().connect().is_err());
        assert!(!ctx.inspector_is_connected());
    }

    #[test]
    fn test_inspector_send_message() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static RECEIVED: AtomicBool = AtomicBool::new(false);
        RECEIVED.store(false, Ordering::SeqCst);

        let ctx = JSContext::new();

        #[inspector_callback]
        fn callback(message: &str) {
            // Verify we receive a response
            if message.contains("\"id\":1") {
                RECEIVED.store(true, Ordering::SeqCst);
            }
        }

        let session = ctx
            .inspector_session()
            .on_message(callback)
            .connect()
            .unwrap();

        // Send a simple Runtime.evaluate command
        session
            .send_message(
            r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#,
            )
            .unwrap();

        assert!(RECEIVED.load(Ordering::SeqCst), "Should receive response");
    }

    #[test]
    fn test_inspector_session_rejects_nul_message() {
        let ctx = JSContext::new();

        #[inspector_callback]
        fn callback(_message: &str) {}

        let session = ctx
            .inspector_session()
            .on_message(callback)
            .connect()
            .unwrap();
        assert!(session.send_message("bad\0message").is_err());
    }

    #[test]
    fn test_inspector_message_wrappers_validate_and_copy() {
        let inbound = InspectorInboundMessage::borrowed(r#"{"id":1}"#);
        assert_eq!(inbound.as_str(), r#"{"id":1}"#);

        let owned = inbound.to_owned_message();
        assert_eq!(owned.as_str(), r#"{"id":1}"#);
        assert_eq!(owned.as_inbound().as_str(), inbound.as_str());
        assert_eq!(owned.as_outbound().unwrap().as_str(), inbound.as_str());

        assert!(matches!(
            InspectorOutboundMessage::borrowed("bad\0message"),
            Err(InspectorMessageError::InteriorNul { position: 3 })
        ));
    }

    #[test]
    fn test_inspector_send_protocol_message() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static RECEIVED: AtomicBool = AtomicBool::new(false);
        RECEIVED.store(false, Ordering::SeqCst);

        let ctx = JSContext::new();

        #[inspector_callback]
        fn callback(message: &str) {
            if InspectorInboundMessage::borrowed(message)
                .as_str()
                .contains("\"id\":3")
            {
                RECEIVED.store(true, Ordering::SeqCst);
            }
        }

        let session = ctx
            .inspector_session()
            .on_message(callback)
            .connect()
            .unwrap();
        let message = InspectorOutboundMessage::borrowed(
            r#"{"id": 3, "method": "Runtime.evaluate", "params": {"expression": "2+2"}}"#,
        )
        .unwrap();

        session.send_protocol_message(message).unwrap();

        assert!(RECEIVED.load(Ordering::SeqCst), "Should receive response");
    }

    #[test]
    fn test_inspector_debugger_enable_disable() {
        use std::sync::atomic::{AtomicU32, Ordering};

        static RESPONSE_COUNT: AtomicU32 = AtomicU32::new(0);
        RESPONSE_COUNT.store(0, Ordering::SeqCst);

        let ctx = JSContext::new();

        #[inspector_callback]
        fn callback(message: &str) {
            if message.contains("\"result\"") {
                RESPONSE_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        let session = ctx
            .inspector_session()
            .on_message(callback)
            .connect()
            .unwrap();

        // Enable debugger
        session
            .send_message(r#"{"id": 1, "method": "Debugger.enable", "params": {}}"#)
            .unwrap();

        // Disable debugger
        session
            .send_message(r#"{"id": 2, "method": "Debugger.disable", "params": {}}"#)
            .unwrap();

        assert!(
            RESPONSE_COUNT.load(Ordering::SeqCst) >= 2,
            "Should receive responses for enable and disable"
        );
    }

    #[test]
    fn test_virtual_module() {
        let ctx = JSContext::new();
        let name = JSValue::string(&ctx, "John Doe");
        let default = JSObject::new(&ctx);
        default
            .set_property("name", &name, Default::default())
            .unwrap();
        let default_value: JSValue = default.into();
        ctx.create_synthetic_module(
            "@rust-jsc",
            &[("default", &default_value), ("name", &name)],
        )
        .unwrap();

        let callbacks = JSAPIModuleLoader {
            moduleLoaderResolve: Some(module_loader_resolve_virtual),
            moduleLoaderEvaluate: None,
            moduleLoaderFetch: None,
            moduleLoaderFetchSource: None,
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
            "virtual_module_default.js",
            None,
        );
        assert!(result.is_ok());
        ctx.run_microtasks();

        let result = ctx.evaluate_script("lib.name", None);

        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value.as_string().unwrap(), "John Doe");

        let result = ctx.evaluate_module_from_source(
            r"
            import { name } from '@rust-jsc';
            globalThis.name = name;
        ",
            "virtual_module_named.js",
            None,
        );
        assert!(result.is_ok());
        ctx.run_microtasks();

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");
    }

    #[test]
    fn test_virtual_module_no_default() {
        let ctx = JSContext::new();
        let name = JSValue::string(&ctx, "John Doe");
        ctx.create_synthetic_module("@rust-jsc", &[("name", &name)])
            .unwrap();

        let callbacks = JSAPIModuleLoader {
            moduleLoaderResolve: Some(module_loader_resolve_virtual),
            moduleLoaderEvaluate: None,
            moduleLoaderFetch: None,
            moduleLoaderFetchSource: None,
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
            "virtual_module_no_default_named.js",
            None,
        );

        assert!(result.is_ok());
        ctx.run_microtasks();

        let result = ctx.evaluate_script("name", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");

        let promise = ctx
            .evaluate_module_from_source(
                r"
            import lib from '@rust-jsc';
            globalThis.lib = lib;
        ",
                "virtual_module_no_default_missing.js",
                None,
            )
            .unwrap();
        ctx.global_object()
            .set_property("__module_promise", &promise, Default::default())
            .unwrap();
        ctx.evaluate_script(
            "__module_promise.catch(error => { globalThis.moduleError = String(error && error.message || error); })",
            None,
        )
        .unwrap();
        ctx.run_microtasks();

        let result = ctx.evaluate_script("globalThis.moduleError", None).unwrap();
        assert!(result.as_string().unwrap().to_string().contains("default"));
    }

    #[test]
    fn test_create_synthetic_module_rejects_duplicate_exports() {
        let ctx = JSContext::new();
        let name = JSValue::string(&ctx, "John Doe");
        let error = ctx
            .create_synthetic_module(
                "@rust-jsc/duplicate",
                &[("name", &name), ("name", &name)],
            )
            .unwrap_err();
        assert!(error
            .message()
            .unwrap()
            .to_string()
            .contains("Duplicate synthetic module export"));
    }

    #[test]
    fn test_create_synthetic_module_rejects_cross_context_values() {
        let ctx = JSContext::new();
        let other = JSContext::new();
        let name = JSValue::string(&other, "John Doe");
        let error = ctx
            .create_synthetic_module("@rust-jsc/cross-context", &[("name", &name)])
            .unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "synthetic module export value belongs to a different context"
        );
    }

    #[test]
    fn test_dynamic_import_synthetic_module() {
        let ctx = JSContext::new();
        let name = JSValue::string(&ctx, "John Doe");
        ctx.create_synthetic_module("@rust-jsc", &[("name", &name)])
            .unwrap();

        let callbacks = JSAPIModuleLoader {
            moduleLoaderResolve: Some(module_loader_resolve_virtual),
            moduleLoaderEvaluate: None,
            moduleLoaderFetch: None,
            moduleLoaderFetchSource: None,
            moduleLoaderCreateImportMetaProperties: Some(
                module_loader_create_import_meta_properties,
            ),
        };
        ctx.set_module_loader(callbacks);

        let promise = ctx
            .evaluate_module_from_source(
                r"
                import('@rust-jsc').then(
                    module => { globalThis.dynamicName = module.name; },
                    error => { globalThis.dynamicError = String(error && error.message || error); },
                );
            ",
                "dynamic_import.js",
                None,
            )
            .unwrap();
        assert!(promise.is_object());
        ctx.run_microtasks();
        ctx.run_microtasks();

        let error = ctx
            .evaluate_script("globalThis.dynamicError", None)
            .unwrap();
        assert!(error.is_undefined(), "dynamic import failed: {error:?}");

        let result = ctx.evaluate_script("globalThis.dynamicName", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "John Doe");
    }

    #[test]
    fn test_non_virtual_module() {
        let ctx = JSContext::new();

        let callbacks = JSAPIModuleLoader {
            moduleLoaderResolve: Some(module_loader_resolve_non_virtual),
            moduleLoaderEvaluate: Some(module_loader_evaluate_virtual),
            moduleLoaderFetch: Some(module_loader_fetch),
            moduleLoaderFetchSource: None,
            moduleLoaderCreateImportMetaProperties: Some(
                module_loader_create_import_meta_properties,
            ),
        };
        ctx.set_module_loader(callbacks);

        let module_test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/modules");
        let test_dir = format!("{}/test.js", module_test_dir);
        let result = ctx.evaluate_module(&test_dir);
        assert!(result.is_ok());
        ctx.run_microtasks();

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
    fn test_set_unhandled_rejection_handler_validates_callable_context() {
        let ctx = JSContext::new();
        let other = JSContext::new();

        let plain_object = JSObject::new(&ctx);
        assert!(ctx.set_unhandled_rejection_handler(&plain_object).is_err());

        let foreign_function = other
            .evaluate_script("function handle() {}; handle", None)
            .unwrap()
            .as_object()
            .unwrap();
        assert!(ctx
            .set_unhandled_rejection_handler(&foreign_function)
            .is_err());
    }

    #[test]
    fn test_set_unhandled_rejection_handler_returns_protected_guard() {
        let ctx = JSContext::new();
        let function = ctx
            .evaluate_script(
                "function handleRejection(promise, reason) { globalThis.unhandledReason = String(reason); }; handleRejection",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();

        let handler = ctx.set_unhandled_rejection_handler(&function).unwrap();
        assert!(handler.function().is_function());

        let (_, resolver) = JSPromise::new_pending(&ctx).unwrap();
        resolver
            .reject(None, &[JSValue::string(&ctx, "boom")])
            .unwrap();
        ctx.run_microtasks();

        let reason = ctx
            .evaluate_script("globalThis.unhandledReason", None)
            .unwrap();
        assert_eq!(reason.as_string().unwrap().to_string(), "boom");
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
        assert!(result.unwrap().is_object());
        ctx.run_microtasks();
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
        if let Some(mut data) = ctx.get_shared_data_mut::<i32>() {
            *data = 20;
        }
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 20);
    }

    #[test]
    fn test_take_shared_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("take me"));
        let taken = ctx.take_shared_data::<String>().unwrap();
        assert_eq!(taken, "take me");
        // After take, data is gone
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_shared_data_none_when_empty() {
        let ctx = JSContext::new();
        assert!(ctx.get_shared_data::<i32>().is_none());
        assert!(ctx.take_shared_data::<i32>().is_none());
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

        let result = ctx.get_shared_data_mut::<String>();
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
            ctx.take_shared_data::<i32>(),
            PrivateDataTakeResult::TypeMismatch
        );
        assert_eq!(
            ctx.take_shared_data::<Vec<u8>>(),
            PrivateDataTakeResult::TypeMismatch
        );

        // Data should still be accessible with correct type
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "preserve me");

        // Now take with correct type
        let taken = ctx.take_shared_data::<String>().unwrap();
        assert_eq!(taken, "preserve me");
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_drop_shared_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("drop me"));

        assert!(ctx.get_shared_data::<String>().is_some());
        assert_eq!(
            ctx.drop_shared_data::<String>(),
            PrivateDataDropStatus::Dropped
        );
        assert!(ctx.get_shared_data::<String>().is_none());
    }

    #[test]
    fn test_drop_shared_data_wrong_type_preserves_data() {
        let ctx = JSContext::new();
        ctx.set_shared_data(String::from("keep me"));

        assert_eq!(
            ctx.drop_shared_data::<i32>(),
            PrivateDataDropStatus::TypeMismatch
        );
        assert_eq!(ctx.get_shared_data::<String>().unwrap(), "keep me");

        assert_eq!(
            ctx.drop_shared_data::<String>(),
            PrivateDataDropStatus::Dropped
        );
        assert_eq!(
            ctx.drop_shared_data::<String>(),
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
            ctx.replace_shared_data(DropCounter {
                drops: drops.clone(),
            }),
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
        let old = ctx.take_shared_data::<i32>().unwrap();
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
        assert_eq!(data.as_slice(), &[1, 2, 3, 4, 5]);
        drop(data);

        let mut data = ctx.get_shared_data_mut::<Vec<u8>>().unwrap();
        data.push(6);
        drop(data);

        let data = ctx.get_shared_data::<Vec<u8>>().unwrap();
        assert_eq!(data.as_slice(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_shared_data_zero_sized_type() {
        let ctx = JSContext::new();
        ctx.set_shared_data(());

        assert!(ctx.get_shared_data::<()>().is_some());
        assert!(ctx.get_shared_data::<i32>().is_none());

        assert!(matches!(
            ctx.take_shared_data::<()>(),
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
            let mut data = ctx.get_shared_data_mut::<i32>().unwrap();
            *data = 200;
        }
        // Reference dropped, now safe to read
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 200);
    }

    #[test]
    fn test_shared_data_guards_block_take_drop_replace_and_mut_aliasing() {
        let ctx = JSContext::new();
        ctx.set_shared_data(10i32);

        let shared = ctx.get_shared_data::<i32>().unwrap();
        assert!(ctx.get_shared_data_mut::<i32>().is_none());
        assert_eq!(
            ctx.take_shared_data::<i32>(),
            PrivateDataTakeResult::Borrowed
        );
        assert_eq!(
            ctx.drop_shared_data::<i32>(),
            PrivateDataDropStatus::Borrowed
        );
        assert_eq!(
            ctx.replace_shared_data(20i32),
            PrivateDataSetStatus::Borrowed
        );
        assert_eq!(*shared, 10);
        drop(shared);

        {
            let mut data = ctx.get_shared_data_mut::<i32>().unwrap();
            assert!(ctx.get_shared_data::<i32>().is_none());
            assert_eq!(
                ctx.take_shared_data::<i32>(),
                PrivateDataTakeResult::Borrowed
            );
            *data = 15;
        }

        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 15);
        assert_eq!(
            ctx.replace_shared_data(20i32),
            PrivateDataSetStatus::Replaced
        );
        assert_eq!(*ctx.get_shared_data::<i32>().unwrap(), 20);
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

        // 2. Taking ownership from an alias now observes the active guard and
        // leaves the allocation in place.
        let taken = ctx_alias.take_shared_data::<String>();
        assert_eq!(taken, PrivateDataTakeResult::Borrowed);
        assert_eq!(data_ref.as_str(), "alive");
        drop(data_ref);

        // Once the guard is dropped, taking ownership succeeds.
        let taken = ctx_alias.take_shared_data::<String>().unwrap();
        assert_eq!(taken, "alive");
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
        drop(data);

        // Take data
        let taken = typed_ctx.take_data().unwrap();
        assert_eq!(taken, "typed data");
        assert!(typed_ctx.get_data().is_none());
    }
}
