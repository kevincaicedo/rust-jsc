use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeDeferredPromise, JSValueRef};

use crate::{
    JSContext, JSError, JSObject, JSPromise, JSPromiseResolvingFunctions, JSResult,
    JSValue,
};

impl JSPromiseResolvingFunctions {
    pub fn resolve(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.resolve.call(this, arguments)
    }

    pub fn reject(
        &self,
        this: Option<&JSObject>,
        arguments: &[JSValue],
    ) -> JSResult<JSValue> {
        self.reject.call(this, arguments)
    }
}

impl JSPromise {
    pub fn new_pending(ctx: &JSContext) -> JSResult<(Self, JSPromiseResolvingFunctions)> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let mut resolve = JSObject::new(ctx);
        let mut reject = JSObject::new(ctx);

        let result = unsafe {
            JSObjectMakeDeferredPromise(
                ctx.inner,
                &mut resolve.inner,
                &mut reject.inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, ctx.inner);
            return Err(JSError::from(value));
        }

        let resolver = JSPromiseResolvingFunctions { resolve, reject };

        Ok((
            Self {
                this: JSObject::from_ref(result, ctx.inner),
                resolver: resolver.clone(),
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

    pub fn then(self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this
            .get_property("then")?
            .as_object()?
            .call(Some(&self.this), arguments)
    }

    pub fn catch(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this
            .get_property("catch")?
            .as_object()?
            .call(Some(&self.this), arguments)
    }

    pub fn finally(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        self.this
            .get_property("finally")?
            .as_object()?
            .call(Some(&self.this), arguments)
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

unsafe impl Send for JSPromise {}

#[cfg(test)]
mod tests {
    use crate::{self as rust_jsc, JSFunction};
    use rust_jsc_macros::callback;

    use crate::{JSContext, JSValue};

    use super::*;

    #[test]
    fn test_new_promise() {
        let ctx = JSContext::new();
        let (promise, _) = JSPromise::new_pending(&ctx).unwrap();
        assert_eq!(promise.is_object(), true);
    }

    #[test]
    fn test_resolve() {
        let ctx = JSContext::new();
        let (promise, _) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.resolve(None, &[value]).unwrap();
        assert_eq!(result.is_undefined(), true);
    }

    #[test]
    fn test_reject() {
        let ctx = JSContext::new();
        let (promise, _) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.reject(None, &[value]).unwrap();
        assert_eq!(result.is_undefined(), true);
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
            let arg = _arguments.get(0).unwrap();
            println!("INFO: {}", arg.as_number().unwrap());

            assert_eq!(arg.as_number().unwrap(), 42.0);
            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let (promise, resolver) = JSPromise::new_pending(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);

        resolver.resolve(None, &[value]).unwrap();
        let function = JSFunction::callback::<String>(&ctx, None, Some(log_info));
        let result = promise.then(&[function.into()]);

        assert_eq!(result.unwrap().is_object(), true);
    }
}
