use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeDeferredPromise, JSObjectRef, JSValueRef};

use crate::{
    not_send_or_sync, JSContext, JSError, JSObject, JSPromise,
    JSPromiseResolvingFunctions, JSResult, JSValue,
};

impl JSPromiseResolvingFunctions {
    fn new(resolve: JSObject, reject: JSObject) -> Self {
        let resolve_protection = resolve.protected();
        let reject_protection = reject.protected();

        Self {
            resolve,
            resolve_protection,
            reject,
            reject_protection,
            _not_send_or_sync: not_send_or_sync(),
        }
    }

    /// Return the JavaScript resolve function.
    pub fn resolve_function(&self) -> &JSObject {
        &self.resolve
    }

    /// Return the JavaScript reject function.
    pub fn reject_function(&self) -> &JSObject {
        &self.reject
    }

    pub fn resolve(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.resolve.call(this, arguments)
    }

    #[deprecated(
        since = "1.0.0",
        note = "promise resolvers are RAII-protected; dropping the resolver releases the protection"
    )]
    pub fn protect(&self) {
        // Resolver functions are protected on construction and unprotected in
        // Drop. This method remains as a no-op for the 1.0 migration window.
    }

    #[deprecated(
        since = "1.0.0",
        note = "promise resolvers are RAII-protected; dropping the resolver releases the protection"
    )]
    pub fn unprotect(&self) {
        // Resolver functions are protected on construction and unprotected in
        // Drop. This method remains as a no-op for the 1.0 migration window.
    }

    pub fn reject(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.reject.call(this, arguments)
    }
}

impl Clone for JSPromiseResolvingFunctions {
    fn clone(&self) -> Self {
        Self {
            resolve: self.resolve.clone(),
            resolve_protection: self.resolve_protection.clone(),
            reject: self.reject.clone(),
            reject_protection: self.reject_protection.clone(),
            _not_send_or_sync: not_send_or_sync(),
        }
    }
}

impl JSPromise {
    pub fn new_pending(ctx: &JSContext) -> JSResult<(Self, JSPromiseResolvingFunctions)> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut resolve: JSObjectRef = std::ptr::null_mut();
        let mut reject: JSObjectRef = std::ptr::null_mut();

        // SAFETY: `ctx.inner` is a live JavaScriptCore context. The out-pointers
        // are valid for this call and are checked before wrapping.
        let result = unsafe {
            JSObjectMakeDeferredPromise(
                ctx.inner,
                &mut resolve,
                &mut reject,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        if result.is_null() || resolve.is_null() || reject.is_null() {
            return Err(JSError::new_typ(
                ctx,
                "failed to create JavaScript Promise resolver",
            )?);
        }

        let resolver = JSPromiseResolvingFunctions::new(
            JSObject::from_ref(resolve, ctx.inner),
            JSObject::from_ref(reject, ctx.inner),
        );

        Ok((
            Self {
                this: JSObject::from_ref(result, ctx.inner),
                resolver: resolver.clone(),
                _not_send_or_sync: not_send_or_sync(),
            },
            resolver,
        ))
    }

    pub fn resolve(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.resolver.resolve.call(this, arguments)
    }

    pub fn reject(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.resolver.reject.call(this, arguments)
    }

    pub fn then(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this.call_method("then", arguments)
    }

    pub fn catch(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this.call_method("catch", arguments)
    }

    pub fn finally(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this.call_method("finally", arguments)
    }
}

impl Deref for JSPromise {
    type Target = JSValue;

    fn deref(&self) -> &JSValue {
        self.this.deref()
    }
}

impl From<JSPromise> for JSObject {
    fn from(promise: JSPromise) -> Self {
        promise.this
    }
}

impl From<JSPromise> for JSValue {
    fn from(promise: JSPromise) -> Self {
        promise.this.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as rust_jsc, JSFunction, JSString};
    use rust_jsc_macros::callback;

    use crate::{JSContext, JSValue};

    use super::*;

    #[test]
    fn test_new_promise() {
        let ctx = JSContext::new();
        let (promise, resolver) = JSPromise::new_pending(&ctx).unwrap();
        assert!(promise.is_object());
        assert!(resolver.resolve_function().is_function());
        assert!(resolver.reject_function().is_function());
    }

    #[test]
    fn test_resolve() {
        let ctx = JSContext::new();
        let (promise, _) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.resolve(None, &[value]).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn test_reject() {
        let ctx = JSContext::new();
        let (promise, _) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.reject(None, &[value]).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn test_promise_keeps_internal_resolver_after_returned_resolver_drop() {
        let ctx = JSContext::new();
        let (promise, resolver) = JSPromise::new_pending(&ctx).unwrap();
        drop(resolver);

        let value = JSValue::number(&ctx, 42.0);
        let result = promise.resolve(None, &[value]).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn test_promise_resolver_clone_keeps_raii_protection() {
        let ctx = JSContext::new();
        let (_, resolver) = JSPromise::new_pending(&ctx).unwrap();
        let cloned = resolver.clone();
        drop(resolver);

        let value = JSValue::number(&ctx, 42.0);
        let result = cloned.resolve(None, &[value]).unwrap();
        assert!(result.is_undefined());
    }

    #[test]
    fn test_resolve_function() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            _arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let arg = _arguments.first().unwrap();
            println!("INFO: {}", arg.as_number().unwrap());

            assert_eq!(arg.as_number().unwrap(), 42.0);
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let (promise, resolver) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);

        resolver.resolve(None, &[value]).unwrap();
        let function = JSFunction::callback::<JSString>(&ctx, None, Some(log_info));
        let result = promise.then(&[function.into()]);

        assert!(result.unwrap().is_object());
    }
}
