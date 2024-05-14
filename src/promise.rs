use std::ops::Deref;

use rust_jsc_sys::{JSObjectMakeDeferredPromise, JSValueRef};

use crate::{JSContext, JSError, JSObject, JSPromise, JSResult, JSValue};

impl JSPromise {
    pub fn new_promise(ctx: &JSContext) -> JSResult<Self> {
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

        Ok(Self {
            this: JSObject::from_ref(result, ctx.inner),
            resolve,
            reject,
        })
    }

    pub fn resolve(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        // Using global object as this
        // TODO: Consider using `self.this` as this or empty object
        self.resolve.call(None, arguments)
    }

    pub fn reject(&self, arguments: &[JSValue]) -> JSResult<JSValue> {
        // Using global object as this
        // TODO: Consider using `self.this` as this or empty object
        self.reject.call(None, arguments)
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
    use crate::{JSContext, JSValue};

    use super::*;

    #[test]
    fn test_new_promise() {
        let ctx = JSContext::new();
        let promise = JSPromise::new_promise(&ctx).unwrap();
        assert_eq!(promise.is_object(), true);
    }

    #[test]
    fn test_resolve() {
        let ctx = JSContext::new();
        let promise = JSPromise::new_promise(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.resolve(&[value]).unwrap();
        assert_eq!(result.is_undefined(), true);
    }

    #[test]
    fn test_reject() {
        let ctx = JSContext::new();
        let promise = JSPromise::new_promise(&ctx).unwrap();
        let value = JSValue::number(&ctx, 42.0);
        let result = promise.reject(&[value]).unwrap();
        assert_eq!(result.is_undefined(), true);
    }
}
