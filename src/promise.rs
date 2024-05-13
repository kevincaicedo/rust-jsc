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
