use std::{iter::FusedIterator, ops::Deref};

use rust_jsc_sys::{
    JSContextRef, JSObjectCallAsConstructor, JSObjectCallAsFunction, JSObjectCallMethod,
    JSObjectCopyPropertyNames, JSObjectDeleteProperty, JSObjectDeletePropertyForKey,
    JSObjectGetPrivate, JSObjectGetProperty, JSObjectGetPropertyAtIndex,
    JSObjectGetPropertyForKey, JSObjectGetPrototype, JSObjectHasProperty,
    JSObjectHasPropertyForKey, JSObjectIsConstructor, JSObjectIsFunction, JSObjectMake,
    JSObjectRef, JSObjectSetAsyncIterator, JSObjectSetIterator, JSObjectSetPrivate,
    JSObjectSetProperty, JSObjectSetPropertyAtIndex, JSObjectSetPropertyForKey,
    JSObjectSetPrototype, JSPropertyNameArrayGetCount, JSPropertyNameArrayGetNameAtIndex,
    JSPropertyNameArrayRef, JSPropertyNameArrayRelease, JSStringRetain, JSValueRef,
};

use crate::{
    with_raw_value_refs, JSContext, JSError, JSObject, JSResult, JSString, JSValue,
    PrivateData, PrivateDataDropStatus, PrivateDataMut, PrivateDataRef,
    PrivateDataSetStatus, PrivateDataTakeResult, PrivateDataWrapper, PropertyDescriptor,
    PropertyKey, ProtectedObject, TryFromJSValue,
};

pub struct JSPropertyNameIter {
    inner: JSPropertyNameArrayRef,
    index: usize,
}

impl JSPropertyNameIter {
    fn remaining(&self) -> usize {
        if self.inner.is_null() {
            return 0;
        }

        // SAFETY: `inner` is either null or an owned
        // `JSPropertyNameArrayRef` returned by `JSObjectCopyPropertyNames`.
        let count = unsafe { JSPropertyNameArrayGetCount(self.inner) };
        count.saturating_sub(self.index)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

impl Iterator for JSPropertyNameIter {
    type Item = JSString;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_null() || self.remaining() == 0 {
            return None;
        }

        // SAFETY: `remaining() > 0` proves `inner` is non-null and `index` is
        // within the JavaScriptCore property-name array bounds.
        let name = unsafe { JSPropertyNameArrayGetNameAtIndex(self.inner, self.index) };
        self.index += 1;
        Some(JSString {
            // SAFETY: the returned name is owned by the property-name array.
            // Retaining it lets the yielded JSString outlive this iterator.
            inner: unsafe { JSStringRetain(name) },
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for JSPropertyNameIter {
    fn len(&self) -> usize {
        self.remaining()
    }
}

impl FusedIterator for JSPropertyNameIter {}

impl Drop for JSPropertyNameIter {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            // SAFETY: `inner` is an owned Create-rule reference from
            // `JSObjectCopyPropertyNames` and is released exactly once here.
            unsafe { JSPropertyNameArrayRelease(self.inner) }
        }
    }
}

impl JSObject {
    /// Creates a new `JSObject` object.
    ///
    /// Creates a new empty JavaScript object.
    pub fn new(ctx: &JSContext) -> Self {
        // SAFETY: `ctx.inner` is a live JavaScriptCore context. Null class and
        // private data request a plain JavaScript object with no Rust payload.
        let inner = unsafe {
            JSObjectMake(ctx.inner, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let value = JSValue::new(inner, ctx.inner);
        Self { inner, value }
    }

    pub(crate) fn from_ref(inner: JSObjectRef, ctx: JSContextRef) -> Self {
        let value = JSValue::new(inner, ctx);
        Self { inner, value }
    }

    /// Creates a JavaScript object wrapper from raw JavaScriptCore handles.
    ///
    /// # Safety
    /// `inner` must be a valid `JSObjectRef` belonging to the live
    /// JavaScriptCore context `ctx`. The returned wrapper does not retain,
    /// protect, or extend the lifetime of either handle, so callers must keep
    /// both handles valid for every use of the wrapper.
    pub unsafe fn from_raw_unchecked(inner: JSObjectRef, ctx: JSContextRef) -> Self {
        Self::from_ref(inner, ctx)
    }

    /// Consumes this object and returns an RAII guard that keeps it protected
    /// from JavaScriptCore garbage collection until the guard is dropped.
    pub fn into_protected(self) -> ProtectedObject {
        ProtectedObject::new(self)
    }

    fn borrowed_context(&self) -> JSContext {
        // SAFETY: `JSObject` stores a borrowed context pointer provided by
        // JavaScriptCore. This helper creates a non-owning view and does not
        // retain or release the context.
        unsafe { JSContext::borrowed(self.value.ctx) }
    }

    fn ensure_value_context(&self, value: &JSValue, role: &str) -> JSResult<()> {
        if value.ctx == self.value.ctx {
            return Ok(());
        }

        Err(JSError::from_message(
            &self.borrowed_context(),
            format!("{role} belongs to a different JavaScript context"),
        ))
    }

    /// Sets a property using a typed [`PropertyKey`].
    ///
    /// This is the high-level property setter for code that wants one API for
    /// string names, JavaScript value keys, and array indexes. Value keys and
    /// values are checked against this object's context before crossing the C
    /// API boundary.
    pub fn set_property_by_key(
        &self,
        key: impl Into<PropertyKey>,
        value: &JSValue,
        descriptor: PropertyDescriptor,
    ) -> JSResult<()> {
        self.ensure_value_context(value, "property value")?;

        match key.into() {
            PropertyKey::String(name) => self.set_property(name, value, descriptor),
            PropertyKey::Value(key) => {
                self.ensure_value_context(&key, "property key")?;
                self.set(&key, value, descriptor)
            }
            PropertyKey::Index(index) => self.set_property_at_index(index, value),
        }
    }

    /// Gets a property using a typed [`PropertyKey`].
    pub fn get_property_by_key(&self, key: impl Into<PropertyKey>) -> JSResult<JSValue> {
        match key.into() {
            PropertyKey::String(name) => self.get_property(name),
            PropertyKey::Value(key) => {
                self.ensure_value_context(&key, "property key")?;
                self.get(&key)
            }
            PropertyKey::Index(index) => self.get_property_at_index(index),
        }
    }

    /// Tests for a property using a typed [`PropertyKey`].
    pub fn has_property_by_key(&self, key: impl Into<PropertyKey>) -> JSResult<bool> {
        match key.into() {
            PropertyKey::String(name) => Ok(self.has_property(name)),
            PropertyKey::Value(key) => {
                self.ensure_value_context(&key, "property key")?;
                self.has(&key)
            }
            PropertyKey::Index(index) => {
                let ctx = self.borrowed_context();
                let key = JSValue::number(&ctx, index as f64);
                self.has(&key)
            }
        }
    }

    /// Deletes a property using a typed [`PropertyKey`].
    pub fn delete_property_by_key(&self, key: impl Into<PropertyKey>) -> JSResult<bool> {
        match key.into() {
            PropertyKey::String(name) => self.delete_property(name),
            PropertyKey::Value(key) => {
                self.ensure_value_context(&key, "property key")?;
                self.delete(&key)
            }
            PropertyKey::Index(index) => {
                let ctx = self.borrowed_context();
                let key = JSValue::number(&ctx, index as f64);
                self.delete(&key)
            }
        }
    }

    /// Sets an object's async iterator.
    /// This function is the same as performing "object[Symbol.asyncIterator] = iterator" from JavaScript.
    /// The iterator object must have a "next" method that returns a promise.
    /// The promise must resolve to an object with a "value" property that contains the next value,
    /// and a "done" property that indicates whether the iterator is done.
    /// The iterator object may have a "return" method that cleans up resources when the iterator is done.
    /// The return method may return a promise.
    ///
    /// Doc: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_async_iterator_and_async_iterable_protocols
    ///
    /// # Arguments
    /// * `iterator` - The iterator object to set on the object.
    /// * `descriptor` - The property descriptor to set on the object.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let iterator = JSObject::new(&ctx);
    ///
    /// object.set_async_iterator(&iterator, PropertyDescriptor::default()).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    ///
    pub fn set_async_iterator(
        &self,
        iterator: &JSObject,
        descriptor: PropertyDescriptor,
    ) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `iterator` are live object handles. JavaScriptCore
        // initializes `exception` if installing the async iterator fails.
        unsafe {
            JSObjectSetAsyncIterator(
                self.ctx,
                self.inner,
                iterator.inner,
                descriptor.attributes,
                &mut exception,
            );
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(())
    }

    /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_async_iterator_and_async_iterable_protocols
    /// Sets an object's iterator.
    /// This function is the same as performing "object[Symbol.iterator] = iterator" from JavaScript.
    /// The iterator object must have a "next" method that returns an object with a "value" property that contains the next value,
    /// and a "done" property that indicates whether the iterator is done.
    /// The iterator object may have a "return" method that cleans up resources when the iterator is done.
    /// The return method may return an object with a "value" property that contains the return value.
    /// The iterator object may have a "throw" method that cleans up resources when the iterator is done.
    ///
    /// # Arguments
    /// * `iterator` - The iterator object to set on the object.
    /// * `descriptor` - The property descriptor to set on the object.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let iterator = JSObject::new(&ctx);
    ///
    /// object.set_iterator(&iterator, PropertyDescriptor::default()).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn set_iterator(
        &self,
        iterator: &JSObject,
        descriptor: PropertyDescriptor,
    ) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `iterator` are live object handles. JavaScriptCore
        // initializes `exception` if installing the iterator fails.
        unsafe {
            JSObjectSetIterator(
                self.ctx,
                self.inner,
                iterator.inner,
                descriptor.attributes,
                &mut exception,
            );
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(())
    }

    /// Tests whether an object has a given property.
    /// Returns true if the object has the property, otherwise false.
    /// This function is the same as performing "property in object" from JavaScript.
    ///
    /// # Arguments
    /// * `name` - The name of the property to test for in the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set_property("name", &value, PropertyDescriptor::default());
    /// assert_eq!(object.has_property("name"), true);
    /// ```
    ///
    /// # Returns
    /// Returns boolean value indicating if the object has the property.
    pub fn has_property(&self, name: impl Into<JSString>) -> bool {
        // SAFETY: `self` is a live object/context pair and the temporary
        // property name is a live `JSStringRef` for the duration of the call.
        unsafe { JSObjectHasProperty(self.value.ctx, self.inner, name.into().inner) }
    }

    /// Gets a property from an object using a JSString as the property key.
    /// Returns the value of the property if it exists, otherwise returns undefined.
    /// This function is the same as performing "object['name']" from JavaScript.
    ///
    /// # Arguments
    /// * `name` - The name of the property to get from the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set_property("name", &value, PropertyDescriptor::default());
    /// assert_eq!(object.get_property("name").unwrap(), value);
    /// ```
    ///
    /// # Returns
    /// Returns the value of the property if it exists, otherwise returns undefined.
    pub fn get_property(&self, name: impl Into<JSString>) -> JSResult<JSValue> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` is a live object/context pair and the temporary
        // property name is a live `JSStringRef`. `exception` is checked below.
        let value = unsafe {
            JSObjectGetProperty(
                self.value.ctx,
                self.inner,
                name.into().inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if value.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(&ctx, "failed to get object property"));
        }

        Ok(JSValue::new(value, self.value.ctx))
    }

    /// Gets a property from an object using an index as the property key
    /// Returns the value of the property if it exists, otherwise returns undefined.
    /// This function is the same as performing \"object[index]\" from JavaScript.
    ///
    /// # Arguments
    /// * `index` - The index of the property to get from the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let array = ctx.evaluate_script("[1, 2, 3]", None).unwrap();
    /// let value = JSValue::string(&ctx, "value");
    /// let array = array.as_object().unwrap();
    ///
    /// array.set_property_at_index(0, &value);
    /// assert_eq!(array.get_property_at_index(0).unwrap(), value);
    /// ```
    ///
    /// # Returns
    /// Returns the value of the property if it exists, otherwise returns undefined.
    pub fn get_property_at_index(&self, index: u32) -> JSResult<JSValue> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` is a live object/context pair. JavaScriptCore
        // initializes `exception` if indexed property access throws.
        let result = unsafe {
            JSObjectGetPropertyAtIndex(self.value.ctx, self.inner, index, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(
                &ctx,
                "failed to get indexed object property",
            ));
        }

        Ok(JSValue::new(result, self.value.ctx))
    }

    /// Sets a property on an object using a JSValue as the property key
    /// This function is the same as performing \"object[propertyKey] = value\" from JavaScript.
    ///
    /// # Arguments
    /// * `key` - The key to set on the object.
    /// * `value` - The value to set on the object.
    /// * `descriptor` - The property descriptor to set on the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let key = JSValue::string(&ctx, "key");
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set(&key, &value, PropertyDescriptor::default()).unwrap();
    /// assert_eq!(object.get(&key).unwrap(), value);
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn set(
        &self,
        key: &JSValue,
        value: &JSValue,
        descriptor: PropertyDescriptor,
    ) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self`, `key`, and `value` are live JavaScriptCore handles.
        // JavaScriptCore initializes `exception` if key conversion or assignment
        // throws.
        unsafe {
            JSObjectSetPropertyForKey(
                self.ctx,
                self.inner,
                key.inner,
                value.inner,
                descriptor.attributes,
                &mut exception,
            );
        }

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(())
    }

    /// Gets a property from an object using a JSValue as the property key
    /// Returns the value of the property if it exists, otherwise returns undefined.
    /// This function is the same as performing \"object[propertyKey]\" from JavaScript.
    ///
    /// # Arguments
    /// * `key` - The key to get from the object.
    ///
    /// # Returns
    /// Returns the value of the property if it exists, otherwise returns undefined.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let key = JSValue::string(&ctx, "key");
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set(&key, &value, PropertyDescriptor::default());
    /// assert_eq!(object.get(&key).unwrap(), value);
    /// ```
    ///
    pub fn get(&self, key: &JSValue) -> JSResult<JSValue> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `key` are live JavaScriptCore handles.
        // JavaScriptCore initializes `exception` if key conversion or property
        // access throws.
        let result = unsafe {
            JSObjectGetPropertyForKey(self.ctx, self.inner, key.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(
                &ctx,
                "failed to get object property for key",
            ));
        }

        Ok(JSValue::new(result, self.ctx))
    }

    /// Tests whether an object has a given property using a JSValue as the property key
    /// Returns true if the object has the property, otherwise false.
    /// This function is the same as performing \"propertyKey in object\" from JavaScript.
    ///
    /// # Arguments
    /// * `key` - The key to test for in the object.
    ///
    /// # Returns
    /// Returns boolean value indicating if the object has the property.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let key = JSValue::string(&ctx, "key");
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set(&key, &value, PropertyDescriptor::default());
    /// assert_eq!(object.has(&key).unwrap(), true);
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn has(&self, key: &JSValue) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `key` are live JavaScriptCore handles.
        // JavaScriptCore initializes `exception` if key conversion throws.
        let result = unsafe {
            JSObjectHasPropertyForKey(self.ctx, self.inner, key.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Deletes a property from an object where the key is a JSValue
    /// Returns true if the delete operation succeeds, otherwise false
    /// (for example, if the property is not configurable).\n
    /// This function is the same as performing \"delete object[propertyKey]\" from JavaScript.
    ///
    /// # Arguments
    /// * `key` - The key to delete from the object.
    ///
    /// # Returns
    /// Returns boolean value indicating if the delete operation succeeded.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let key = JSValue::string(&ctx, "key");
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set(&key, &value, PropertyDescriptor::default());
    /// assert_eq!(object.has(&key).unwrap(), true);
    /// assert_eq!(object.delete(&key).unwrap(), true);
    /// assert_eq!(object.has(&key).unwrap(), false);
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the delete operation fails.
    pub fn delete(&self, key: &JSValue) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `key` are live JavaScriptCore handles.
        // JavaScriptCore initializes `exception` if key conversion or deletion
        // throws.
        let result = unsafe {
            JSObjectDeletePropertyForKey(self.ctx, self.inner, key.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Sets a property on an object using a JSString as the property key
    /// This function is the same as performing \"object['propertyKey'] = value\" from JavaScript.
    ///
    /// # Arguments
    /// * `name` - The name of the property to set on the object.
    /// * `value` - The value to set on the object.
    /// * `descriptor` - The property descriptor to set on the object.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set_property("name", &value, PropertyDescriptor::default()).unwrap();
    /// assert_eq!(object.get_property("name").unwrap(), value);
    /// ```
    pub fn set_property(
        &self,
        name: impl Into<JSString>,
        value: &JSValue,
        descriptor: PropertyDescriptor,
    ) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self`, `value`, and the temporary property name are live
        // JavaScriptCore handles. JavaScriptCore initializes `exception` if
        // assignment throws.
        unsafe {
            JSObjectSetProperty(
                self.value.ctx,
                self.inner,
                name.into().inner,
                value.inner,
                descriptor.attributes,
                &mut exception,
            );
        }

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(())
    }

    /// Sets a property on an object using an index as the property key
    /// This function is the same as performing \"object[index] = value\" from JavaScript.
    ///
    /// # Arguments
    /// * `index` - The index of the property to set on the object.
    /// * `value` - The value to set on the object.
    ///
    /// # Example
    /// ```
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let array = ctx.evaluate_script("[1, 2, 3]", None).unwrap();
    /// let value = JSValue::string(&ctx, "value");
    /// let array = array.as_object().unwrap();
    ///
    /// array.set_property_at_index(0, &value);
    /// assert_eq!(array.get_property_at_index(0).unwrap(), value);
    /// ```
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn set_property_at_index(&self, index: u32, value: &JSValue) -> JSResult<()> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and `value` are live JavaScriptCore handles.
        // JavaScriptCore initializes `exception` if indexed assignment throws.
        unsafe {
            JSObjectSetPropertyAtIndex(
                self.value.ctx,
                self.inner,
                index,
                value.inner,
                &mut exception,
            );
        }

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(())
    }

    /// Deletes a property from an object where the key is a JSString
    /// Returns true if the delete operation succeeds, otherwise false
    /// (for example, if the property is not configurable).\n
    /// This function is the same as performing \"delete object['propertyKey']\" from JavaScript.
    ///
    /// # Arguments
    /// * `name` - The name of the property to delete from the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set_property("name", &value, PropertyDescriptor::default());
    /// assert_eq!(object.has_property("name"), true);
    /// assert_eq!(object.delete_property("name").unwrap(), true);
    /// assert_eq!(object.has_property("name"), false);
    /// ```
    ///
    /// # Returns
    /// Returns boolean value indicating if the delete operation succeeded.
    pub fn delete_property(&self, name: impl Into<JSString>) -> JSResult<bool> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        // SAFETY: `self` and the temporary property name are live JavaScriptCore
        // handles. JavaScriptCore initializes `exception` if deletion throws.
        let result = unsafe {
            JSObjectDeleteProperty(
                self.value.ctx,
                self.inner,
                name.into().inner,
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(result)
    }

    /// Returns an iterator over the property names of the object.
    /// The iterator will yield `JSString` objects.
    /// The order of the property names is not guaranteed.
    /// The iterator will be deallocated when it goes out of scope.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let key = JSValue::string(&ctx, "key");
    /// let value = JSValue::string(&ctx, "value");
    ///
    /// object.set(&key, &value, PropertyDescriptor::default());
    ///
    /// for name in object.get_property_names() {
    ///    println!("Property name: {}", name);
    /// }
    /// ```
    ///
    /// # Returns
    /// Returns an iterator over the property names of the object.
    pub fn get_property_names(&self) -> JSPropertyNameIter {
        let property_name_array =
            // SAFETY: `self` is a live object/context pair. JavaScriptCore
            // returns an owned property-name array released by the iterator.
            unsafe { JSObjectCopyPropertyNames(self.value.ctx, self.inner) };
        JSPropertyNameIter {
            inner: property_name_array,
            index: 0,
        }
    }

    /// Gets an object's prototype.
    /// This function is the same as performing "Object.getPrototypeOf(object)" from JavaScript.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let prototype = object.get_prototype();
    /// println!("Object's prototype: {:?}", prototype);
    /// ```
    ///
    /// # Returns
    /// JSValue that is the object's prototype.
    pub fn get_prototype(&self) -> JSValue {
        JSValue::new(
            // SAFETY: `self` is a live object/context pair. JavaScriptCore
            // returns a borrowed prototype value in the same context.
            unsafe { JSObjectGetPrototype(self.value.ctx, self.inner) },
            self.value.ctx,
        )
    }

    /// Sets an object's prototype, panicking if the prototype belongs to a
    /// different JavaScript context.
    ///
    /// Prefer [`JSObject::set_prototype_checked`] when the prototype is not
    /// known to come from the same context.
    ///
    /// # Arguments
    /// * `prototype` - The prototype to set on the object.
    ///
    /// # Panics
    /// Panics if `prototype` belongs to a different JavaScript context.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let prototype = JSObject::new(&ctx);
    /// object.set_prototype(&prototype);
    /// ```
    pub fn set_prototype(&self, prototype: &JSObject) {
        self.set_prototype_checked(prototype)
            .expect("prototype object must belong to the same JavaScript context");
    }

    /// Sets an object's prototype after validating context affinity.
    ///
    /// This function is the checked equivalent of
    /// `Object.setPrototypeOf(object, prototype)` for Rust code that may be
    /// assembling objects from dynamic contexts.
    ///
    /// # Errors
    /// Returns a [`JSError`] if `prototype` belongs to a different JavaScript
    /// context.
    pub fn set_prototype_checked(&self, prototype: &JSObject) -> JSResult<()> {
        self.ensure_value_context(&prototype.value, "prototype object")?;

        // SAFETY: `self` and `prototype` are live JavaScriptCore object
        // handles, and the context check above proves that the prototype is
        // being installed within the object's JavaScript context.
        unsafe {
            JSObjectSetPrototype(self.ctx, self.inner, prototype.inner);
        }

        Ok(())
    }

    /// Sets private data on an object.
    /// The default object class does not allocate storage for private data.
    /// Only objects created with a non-NULL JSClass can store private data.
    ///
    /// This method is unsafe because it would overwrite any existing private data on the object,
    /// leaking memory if the existing private data is not properly cleaned up.
    /// it could also use a different type than the one originally stored, in the JSClass and finalize callbacks
    /// use it only if you know what you're doing.
    ///
    ///
    /// # Arguments
    /// * `data` - The private data to set on the object.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let class = JSClass::builder("Data").build::<i32>().unwrap();
    /// let object = class.object::<i32>(&ctx, None).as_object().unwrap();
    /// // SAFETY: the object was created with a class/finalizer that stores `i32`.
    /// unsafe { object.set_private_data(42i32) }.unwrap();
    ///
    /// let private_data = object.get_private_data::<i32>().unwrap();
    /// assert_eq!(*private_data, 42);
    /// ```
    ///
    /// # Safety
    /// The caller must ensure no references into the old private data are alive
    /// if this call replaces an existing Rust-owned pointer. The object's class
    /// finalizer must also agree with the stored Rust type.
    ///
    /// # Returns
    /// Returns [`PrivateDataSetStatus::Set`] or [`PrivateDataSetStatus::Replaced`]
    /// if object can store private data, otherwise [`PrivateDataSetStatus::Unsupported`].
    pub unsafe fn set_private_data<T: 'static>(&self, data: T) -> PrivateDataSetStatus {
        // SAFETY: `self.inner` is a live JavaScriptCore object. The raw private
        // pointer is only inspected through `PrivateDataWrapper` below.
        let old_ptr = unsafe { JSObjectGetPrivate(self.inner) };
        // SAFETY: `old_ptr` is either null or a Rust private-data pointer
        // previously installed by this API; borrowed data must not be replaced.
        if unsafe { PrivateDataWrapper::is_borrowed(old_ptr) } {
            return PrivateDataSetStatus::Borrowed;
        }

        let data_ptr = PrivateDataWrapper::into_raw(data);
        // SAFETY: `data_ptr` is a fresh Rust allocation. The caller upholds the
        // object/class finalizer type contract for storing it in this object.
        let success = unsafe { JSObjectSetPrivate(self.inner, data_ptr) };
        if !success {
            // If the object cannot store private data, we must free the allocation
            // to prevent a memory leak.
            // SAFETY: JavaScriptCore rejected `data_ptr`, so Rust still owns the
            // allocation and must drop it.
            unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
            return PrivateDataSetStatus::Unsupported;
        }

        if !old_ptr.is_null() {
            // SAFETY: `old_ptr` was the previous Rust-owned allocation and no
            // runtime borrow is active, checked above.
            unsafe { PrivateDataWrapper::drop_erased(old_ptr) };
            return PrivateDataSetStatus::Replaced;
        }

        PrivateDataSetStatus::Set
    }

    /// Gets the private data from an object as an immutable guard.
    ///
    /// Returns `None` if no private data is set or if `T` does not match
    /// the type that was originally stored with [`set_private_data`]. It also
    /// returns `None` while the data is mutably borrowed.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// object.set_private_data(42i32);
    ///
    /// let private_data = object.get_private_data::<i32>().unwrap();
    /// assert_eq!(*private_data, 42);
    /// ```
    ///
    /// # Type Safety
    /// Requesting the wrong type returns `None` instead of causing UB:
    /// ```rust,ignore
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// object.set_private_data(String::from("hello"));
    ///
    /// assert!(object.get_private_data::<u64>().is_none()); // wrong type → None
    /// assert!(object.get_private_data::<String>().is_some()); // correct type → Some
    /// ```
    ///
    /// # Returns
    /// Returns a guard to the private data if it exists and type matches, otherwise None.
    pub fn get_private_data<T: 'static>(&self) -> Option<PrivateDataRef<'_, T>> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };
        // SAFETY: the pointer is either null or one installed by the private-data
        // APIs; `borrow_ref` validates type and runtime borrow state.
        unsafe { PrivateDataWrapper::borrow_ref(data_ptr) }
    }

    /// Gets the private data from an object and takes ownership of it.
    /// This will remove the private data from the object, leaving it with no private data.
    ///
    /// If any shared or mutable private-data borrow is active, this method
    /// returns [`PrivateDataTakeResult::Borrowed`] and leaves the data in place.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let class = JSClass::builder("Data").build::<i32>().unwrap();
    /// let object = class.object::<i32>(&ctx, None).as_object().unwrap();
    /// // SAFETY: the object was created with a class/finalizer that stores `i32`.
    /// unsafe { object.set_private_data(42i32) }.unwrap();
    ///
    /// let private_data = object.take_private_data::<i32>().unwrap();
    /// assert_eq!(private_data, 42);
    /// assert!(object.get_private_data::<i32>().is_none()); // private data has been taken, so it should be None
    /// ```
    ///
    /// # Type Safety
    /// Requesting the wrong type returns `None` instead of causing UB.
    /// On type mismatch, the data stays in place — nothing is freed or lost.
    pub fn take_private_data<T: 'static>(&self) -> PrivateDataTakeResult<T> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };
        // SAFETY: the pointer is either null or one installed by the private-data
        // APIs; `take` validates type and refuses active borrows.
        let data = unsafe { PrivateDataWrapper::take(data_ptr) };
        if data.is_taken() {
            // SAFETY: ownership was taken and the allocation is gone, so the
            // JavaScriptCore private slot must be cleared.
            unsafe { JSObjectSetPrivate(self.inner, std::ptr::null_mut()) };
        }
        data
    }

    /// Gets the private data from an object as an exclusive mutable guard.
    ///
    /// Returns `None` if no private data is set or if `T` does not match
    /// the type that was originally stored with [`set_private_data`]. It also
    /// returns `None` while any shared or mutable private-data borrow is active.
    ///
    /// # Recommended alternative
    ///
    /// For safe interior mutability, store a [`std::cell::RefCell<T>`] and use
    /// [`get_private_data`] instead.
    pub fn get_private_data_mut<T: 'static>(&self) -> Option<PrivateDataMut<'_, T>> {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };
        // SAFETY: the pointer is either null or one installed by the private-data
        // APIs; `borrow_mut` validates type and exclusive borrow state.
        unsafe { PrivateDataWrapper::borrow_mut(data_ptr) }
    }

    /// Drops the private data without reclaiming ownership.
    /// This is useful for cleaning up data when the object is being dropped, without needing to take ownership of it.
    /// After this call, the object's private data pointer is cleared only when
    /// the stored type matches `T`.
    ///
    /// If the type does not match, this method returns
    /// [`PrivateDataDropStatus::TypeMismatch`] and leaves the data pointer intact.
    /// If any shared or mutable private-data borrow is active, this method
    /// returns [`PrivateDataDropStatus::Borrowed`] and leaves the data in place.
    pub fn drop_private_data<T: 'static>(&self) -> PrivateDataDropStatus {
        // SAFETY: `self.inner` is live. JavaScriptCore only returns the stored
        // opaque pointer; Rust validates type and borrow state below.
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };
        // SAFETY: the pointer is either null or one installed by the private-data
        // APIs; `drop_raw` validates `T` and refuses active borrows.
        let status = unsafe { PrivateDataWrapper::drop_raw::<T>(data_ptr) };
        if status.is_dropped() {
            // SAFETY: the allocation was dropped, so the JavaScriptCore private
            // slot must be cleared.
            unsafe { JSObjectSetPrivate(self.inner, std::ptr::null_mut()) };
        }
        status
    }

    pub fn get_private_data_ptr(&self) -> Option<PrivateData> {
        // SAFETY: `self.inner` is live. The raw pointer is returned opaquely and
        // must only be interpreted by typed private-data APIs.
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };

        if data_ptr.is_null() {
            return None;
        }

        Some(data_ptr)
    }

    /// Tests whether an object is a constructor.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    ///
    /// assert_eq!(object.is_constructor(), false);
    /// ```
    ///
    /// # Returns
    /// Returns true if the object can be called as a constructor, otherwise false.
    pub fn is_constructor(&self) -> bool {
        // SAFETY: `self` is a live object/context pair and the C API only
        // observes whether it can be called as a constructor.
        unsafe { JSObjectIsConstructor(self.value.ctx, self.inner) }
    }

    /// Deprecated misspelled alias for [`JSObject::is_constructor`].
    #[deprecated(
        since = "1.0.0",
        note = "use is_constructor; is_contructor will be removed after the 1.0 migration window"
    )]
    pub fn is_contructor(&self) -> bool {
        self.is_constructor()
    }

    /// Tests whether an object is a function.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    ///
    /// assert_eq!(object.is_function(), false);
    /// ```
    ///
    /// # Returns
    /// Returns true if the object is a function, otherwise false.
    pub fn is_function(&self) -> bool {
        // SAFETY: `self` is a live object/context pair and the C API only
        // observes whether it can be called as a function.
        unsafe { JSObjectIsFunction(self.value.ctx, self.inner) }
    }

    /// Calls an object as a constructor.
    ///
    /// # Arguments
    /// * `args` - The arguments to pass to the constructor.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let result = object.call_as_constructor(&[]).unwrap();
    /// ```
    ///
    /// # Returns
    /// Returns a result JSObject of calling the object as a constructor.
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn call_as_constructor(&self, args: &[JSValue]) -> JSResult<Self> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `self` is a live JavaScript object and `arguments` points
            // to `argument_count` raw JS values for the duration of this call.
            unsafe {
                JSObjectCallAsConstructor(
                    self.value.ctx,
                    self.inner,
                    argument_count,
                    arguments,
                    &mut exception,
                )
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(
                &ctx,
                "failed to call object as constructor",
            ));
        }

        Ok(JSObject::from_ref(result, self.value.ctx))
    }

    /// Calls an object as a constructor and converts the constructed object.
    ///
    /// This uses [`JSObject::call_as_constructor`] for JavaScriptCore exception
    /// behavior, then converts the result through [`TryFromJSValue`].
    pub fn call_as_constructor_typed<T>(&self, args: &[JSValue]) -> JSResult<T>
    where
        T: TryFromJSValue,
    {
        let object = self.call_as_constructor(args)?;
        T::try_from_js_value(&object.value)
    }

    /// Calls an object as a function.
    ///
    /// # Arguments
    /// * `this` - The object to use as `this` when calling the function.
    /// * `args` - The arguments to pass to the function.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let result = object.call(None, &[]).unwrap();
    /// ```
    ///
    /// # Returns
    /// Returns a result JSValue of calling the object as a function.
    ///
    /// # Errors
    /// Returns a `JSError` if the operation fails.
    pub fn call(&self, this: Option<&JSObject>, args: &[JSValue]) -> JSResult<JSValue> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let this_object = this.map_or(std::ptr::null_mut(), |this| this.inner);
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `self` and `this_object` are live JavaScript objects in
            // this context, and `arguments` points to `argument_count` raw JS
            // values for the duration of this call.
            unsafe {
                JSObjectCallAsFunction(
                    self.value.ctx,
                    self.inner,
                    this_object,
                    argument_count,
                    arguments,
                    &mut exception,
                )
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(
                &ctx,
                "failed to call object as function",
            ));
        }

        Ok(JSValue::new(result, self.value.ctx))
    }

    /// Calls an object as a function and converts the return value.
    ///
    /// This uses [`JSObject::call`] for JavaScriptCore exception behavior, then
    /// converts the result through [`TryFromJSValue`].
    pub fn call_typed<T>(&self, this: Option<&JSObject>, args: &[JSValue]) -> JSResult<T>
    where
        T: TryFromJSValue,
    {
        let value = self.call(this, args)?;
        T::try_from_js_value(&value)
    }

    /// Calls a named method on this object with `self` as `this`.
    ///
    /// This performs the same observable method lookup as JavaScript
    /// (`object[name](...)`) while avoiding a separate Rust-side property get
    /// and function call through the C API.
    ///
    /// # Errors
    /// Returns a `JSError` if the method lookup throws, the property is not
    /// callable, or the method call throws.
    pub fn call_method(
        &self,
        name: impl Into<JSString>,
        args: &[JSValue],
    ) -> JSResult<JSValue> {
        let mut exception: JSValueRef = std::ptr::null_mut();
        let name = name.into();
        let result = with_raw_value_refs(args, |argument_count, arguments| {
            // SAFETY: `self` and `name` are live JavaScriptCore handles, and
            // `arguments` points to `argument_count` raw JS values for the
            // duration of this call.
            unsafe {
                JSObjectCallMethod(
                    self.value.ctx,
                    self.inner,
                    name.inner,
                    argument_count,
                    arguments,
                    &mut exception,
                )
            }
        });

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        if result.is_null() {
            let ctx = self.borrowed_context();
            return Err(JSError::from_message(&ctx, "failed to call object method"));
        }

        Ok(JSValue::new(result, self.value.ctx))
    }

    /// Calls a named method and converts the return value.
    ///
    /// This uses [`JSObject::call_method`] for JavaScriptCore exception
    /// behavior, then converts the result through [`TryFromJSValue`].
    pub fn call_method_typed<T>(
        &self,
        name: impl Into<JSString>,
        args: &[JSValue],
    ) -> JSResult<T>
    where
        T: TryFromJSValue,
    {
        let value = self.call_method(name, args)?;
        T::try_from_js_value(&value)
    }
}

impl std::fmt::Debug for JSObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSObject").finish()
    }
}

impl Deref for JSObject {
    type Target = JSValue;

    fn deref(&self) -> &JSValue {
        &self.value
    }
}

impl From<JSObject> for JSValue {
    fn from(object: JSObject) -> Self {
        object.value
    }
}

impl From<JSObject> for JSObjectRef {
    fn from(object: JSObject) -> Self {
        object.inner
    }
}

#[cfg(test)]
mod tests {

    use crate::{self as rust_jsc, JSString};
    use rust_jsc_macros::callback;

    use crate::{
        JSContext, JSFunction, JSObject, JSResult, JSValue, PropertyDescriptor,
        PropertyKey,
    };

    #[test]
    fn test_object() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get(&key).unwrap(), value);
        assert!(object.has(&key).unwrap());
        assert!(object.delete(&key).unwrap());
        assert!(!object.has(&key).unwrap());
    }

    #[test]
    fn test_object_property() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let name = "name";
        let value = JSValue::string(&ctx, "value");

        object
            .set_property(name, &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get_property(name).unwrap(), value);
        assert!(object.has_property(name));
        assert!(object.delete_property(name).unwrap());
        assert!(!object.has_property(name));
    }

    #[test]
    fn test_object_prototype() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let prototype = JSObject::new(&ctx);

        object.set_prototype(&prototype);
        assert_eq!(object.get_prototype(), prototype.into());
    }

    #[test]
    fn test_object_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let result = object.call_as_constructor(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_object_function() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert!(!object.is_function());

        let function = ctx
            .evaluate_script("function test() { return 42; }; test", None)
            .unwrap();
        assert!(function.is_object());

        let function = function.as_object().unwrap();
        assert!(function.is_function());

        let result = function.call(None, &[]).unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);
    }

    #[test]
    fn test_object_property_names() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();

        let mut property_names = object.get_property_names();
        assert_eq!(property_names.len(), 1);
        assert_eq!(property_names.size_hint(), (1, Some(1)));
        assert!(!property_names.is_empty());
        assert_eq!(property_names.next(), Some(JSString::from("key")));
        assert_eq!(property_names.len(), 0);
        assert_eq!(property_names.size_hint(), (0, Some(0)));
        assert!(property_names.is_empty());
        assert_eq!(property_names.next(), None);
    }

    #[test]
    fn test_object_property_at_index() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "value");

        object.set_property_at_index(0, &value).unwrap();
        assert_eq!(object.get_property_at_index(0).unwrap(), value);
    }

    #[test]
    fn test_object_set_property() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "value");

        object
            .set_property("name", &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get_property("name").unwrap(), value);
    }

    #[test]
    fn test_object_property_key_api_handles_string_value_and_index_keys() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let string_value = JSValue::string(&ctx, "string-value");
        let symbol_key = PropertyKey::symbol(&ctx, "slot");
        let symbol_value = JSValue::string(&ctx, "symbol-value");
        let indexed_value = JSValue::string(&ctx, "indexed-value");

        object
            .set_property_by_key("name", &string_value, PropertyDescriptor::default())
            .unwrap();
        object
            .set_property_by_key(
                symbol_key.clone(),
                &symbol_value,
                PropertyDescriptor::builder().non_enumerable().build(),
            )
            .unwrap();
        object
            .set_property_by_key(
                PropertyKey::index(0),
                &indexed_value,
                Default::default(),
            )
            .unwrap();

        assert_eq!(object.get_property_by_key("name").unwrap(), string_value);
        assert_eq!(
            object.get_property_by_key(symbol_key.clone()).unwrap(),
            symbol_value
        );
        assert_eq!(object.get_property_by_key(0_u32).unwrap(), indexed_value);
        assert!(object.has_property_by_key("name").unwrap());
        assert!(object.has_property_by_key(symbol_key.clone()).unwrap());
        assert!(object.has_property_by_key(0_u32).unwrap());
        assert!(object.delete_property_by_key("name").unwrap());
        assert!(object.delete_property_by_key(symbol_key).unwrap());
        assert!(object.delete_property_by_key(0_u32).unwrap());
    }

    #[test]
    fn test_object_property_key_api_rejects_cross_context_values() {
        let ctx = JSContext::new();
        let other_ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&other_ctx, "key");
        let value = JSValue::string(&ctx, "value");
        let other_value = JSValue::string(&other_ctx, "value");

        assert!(object
            .set_property_by_key(&key, &value, PropertyDescriptor::default())
            .is_err());
        assert!(object
            .set_property_by_key("name", &other_value, PropertyDescriptor::default())
            .is_err());
        assert!(object.get_property_by_key(&key).is_err());
    }

    #[test]
    fn test_object_property_key_api_propagates_value_key_conversion_exceptions() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let throwing_key = ctx
            .evaluate_script(
                "({ [Symbol.toPrimitive]() { throw new TypeError('key failed'); } })",
                None,
            )
            .unwrap();
        let value = JSValue::string(&ctx, "value");

        assert!(object
            .set_property_by_key(&throwing_key, &value, PropertyDescriptor::default(),)
            .is_err());
        assert!(object.get_property_by_key(&throwing_key).is_err());
        assert!(object.has_property_by_key(&throwing_key).is_err());
        assert!(object.delete_property_by_key(&throwing_key).is_err());
    }

    #[test]
    fn test_object_set_property_propagates_exception() {
        let ctx = JSContext::new();
        let object = ctx
            .evaluate_script(
                "({ set name(_) { throw new TypeError('setter failed'); } })",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();
        let value = JSValue::string(&ctx, "value");

        let error = object
            .set_property("name", &value, PropertyDescriptor::default())
            .unwrap_err();

        assert_eq!(error.name().unwrap().to_string(), "TypeError");
        assert_eq!(error.message().unwrap().to_string(), "setter failed");
    }

    #[test]
    fn test_object_set_property_at_index() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "value");

        object.set_property_at_index(0, &value).unwrap();
        assert_eq!(object.get_property_at_index(0).unwrap(), value);
    }

    #[test]
    fn test_object_delete_property() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let name = "name";
        let value = JSValue::string(&ctx, "value");

        object
            .set_property(name, &value, PropertyDescriptor::default())
            .unwrap();
        assert!(object.has_property(name));
        assert!(object.delete_property(name).unwrap());
        assert!(!object.has_property(name));
    }

    #[test]
    fn test_object_has_property() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let name = JSString::from("name");
        let value = JSValue::string(&ctx, "value");

        object
            .set_property(name, &value, PropertyDescriptor::default())
            .unwrap();
        assert!(object.has_property("name"));
    }

    #[test]
    fn test_object_get_property() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "value");

        object
            .set_property("name", &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get_property("name").unwrap(), value);
    }

    #[test]
    fn test_object_get_property_at_index() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let value = JSValue::string(&ctx, "value");

        object.set_property_at_index(0, &value).unwrap();
        assert_eq!(object.get_property_at_index(0).unwrap(), value);
    }

    #[test]
    fn test_object_set() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get(&key).unwrap(), value);
    }

    #[test]
    fn test_object_get() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();
        assert_eq!(object.get(&key).unwrap(), value);
    }

    #[test]
    fn test_object_has() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();
        assert!(object.has(&key).unwrap());
    }

    #[test]
    fn test_object_delete() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();
        assert!(object.has(&key).unwrap());
        assert!(object.delete(&key).unwrap());
        assert!(!object.has(&key).unwrap());
    }

    #[test]
    fn test_object_get_prototype() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let prototype = JSObject::new(&ctx);

        object.set_prototype(&prototype);

        assert_eq!(object.get_prototype(), prototype.into());
    }

    #[test]
    fn test_object_set_prototype() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let prototype = JSObject::new(&ctx);

        object.set_prototype(&prototype);
        assert_eq!(object.get_prototype(), prototype.into());
    }

    #[test]
    fn test_object_set_prototype_checked_rejects_cross_context() {
        let ctx = JSContext::new();
        let other_ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let prototype = JSObject::new(&other_ctx);

        let error = object.set_prototype_checked(&prototype).unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "prototype object belongs to a different JavaScript context"
        );
    }

    #[test]
    fn test_object_is_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert!(!object.is_constructor());
    }

    #[test]
    fn test_object_is_function() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert!(!object.is_function());
    }

    #[test]
    fn test_object_call_as_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let result = object.call_as_constructor(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_object_call_method() {
        let ctx = JSContext::new();
        let object = ctx
            .evaluate_script(
                "({ base: 5, add(value) { return this.base + value; } })",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();

        let result = object
            .call_method("add", &[JSValue::number(&ctx, 37.0)])
            .unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);

        let typed_result: f64 = object
            .call_method_typed("add", &[JSValue::number(&ctx, 8.0)])
            .unwrap();
        assert_eq!(typed_result, 13.0);

        let error = object.call_method("missing", &[]).unwrap_err();
        assert_eq!(
            error.message().unwrap().to_string(),
            "Object method property should be callable"
        );
    }

    #[test]
    fn test_protected_object_keeps_callable_alive_and_clone_safe() {
        let ctx = JSContext::new();
        let function = ctx
            .evaluate_script("(function(value) { return value + 1; })", None)
            .unwrap()
            .as_object()
            .unwrap()
            .into_protected();
        let cloned = function.clone();

        ctx.garbage_collect();
        assert_eq!(
            function
                .call(None, &[JSValue::number(&ctx, 41.0)])
                .unwrap()
                .as_number()
                .unwrap(),
            42.0
        );

        drop(function);
        ctx.garbage_collect();
        assert_eq!(
            cloned
                .call(None, &[JSValue::number(&ctx, 1.0)])
                .unwrap()
                .as_number()
                .unwrap(),
            2.0
        );
    }

    #[test]
    fn test_object_debug() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert_eq!(format!("{:?}", object), "JSObject".to_string());
    }

    #[test]
    fn test_object_property_names_iter() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let key = JSValue::string(&ctx, "key");
        let value = JSValue::string(&ctx, "value");

        object
            .set(&key, &value, PropertyDescriptor::default())
            .unwrap();

        let property_names: Vec<_> = object.get_property_names().collect();
        assert_eq!(property_names, vec![JSString::from("key")]);
    }

    #[test]
    fn test_iterator() {
        #[callback]
        fn log_info(
            ctx: JSContext,
            _function: JSObject,
            _this: JSObject,
            arguments: &[JSValue],
        ) -> JSResult<JSValue> {
            let message = arguments.first().unwrap().as_string().unwrap();
            println!("INFO: {}", message);

            Ok(JSValue::undefined(&ctx))
        }

        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let iterator = r#"
        const myIterator = () => {
            let i = 0;
            return {
              next() {
                i++;
                console.log(`Returning ${i}`);
                if (i === 4) return { done: true };
                return { done: false, value: i };
              },
              return() {
                console.log("Closing");
                return { done: true };
              },
            };
        };
        myIterator
        "#;
        let iterator_object = ctx
            .evaluate_script(iterator, None)
            .unwrap()
            .as_object()
            .unwrap();
        object
            .set_iterator(&iterator_object, PropertyDescriptor::default())
            .unwrap();

        let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
        object
            .set_property("log", &function, Default::default())
            .unwrap();
        ctx.global_object()
            .set_property("console", &object, Default::default())
            .unwrap();
        ctx.global_object()
            .set_property("myObjectIter", &object, PropertyDescriptor::default())
            .unwrap();

        let evaluate_script = r#"
        let counter = 0;
        for (let i of myObjectIter) {
            console.log(i);
            counter += i;
        }
        counter
        "#;

        let result = ctx.evaluate_script(evaluate_script, None);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.as_number().unwrap(), 6.0);
    }

    #[test]
    fn test_async_iterator() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let async_iterator = r#"
        const myAsyncIterator = () => {
            let i = 0;
            return {
              async next() {
                i++;
                console.log(`Returning ${i}`);
                if (i === 4) return { done: true };
                return { done: false, value: i };
              },
              async return() {
                console.log("Closing");
                return { done: true };
              },
            };
        };
        myAsyncIterator
        "#;

        let async_iterator_object = ctx
            .evaluate_script(async_iterator, None)
            .unwrap()
            .as_object()
            .unwrap();
        object
            .set_async_iterator(&async_iterator_object, PropertyDescriptor::default())
            .unwrap();
        ctx.global_object()
            .set_property("myObjectIter", &object, PropertyDescriptor::default())
            .unwrap();

        let evaluate_script = r#"
        let counter = 0;
        (async function () {
            for await (let i of myObjectIter) {
                console.log(i);
                counter += i;
            }
        })();
        "#;

        let result = ctx.evaluate_script(evaluate_script, None);

        assert!(result.is_ok());
    }

    // =========================================================================
    // Private Data Tests
    // =========================================================================

    #[test]
    fn test_object_private_data_not_available_on_plain_object() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);

        // Plain objects (created without a JSClass) cannot store private data
        assert!(object.get_private_data::<i32>().is_none());
    }

    #[test]
    fn test_object_private_data_wrong_type_returns_none() {
        use crate::JSClass;

        let ctx = JSContext::new();
        let class = JSClass::builder("WrongTypeObj").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("data")));
        let object = object.as_object().unwrap();

        assert!(object.get_private_data::<i32>().is_none());
        assert!(object.get_private_data::<u64>().is_none());
        assert!(object.get_private_data::<Vec<u8>>().is_none());
        assert_eq!(object.get_private_data::<String>().unwrap(), "data");
    }

    #[test]
    fn test_object_take_private_data_type_safe() {
        use crate::{JSClass, PrivateDataTakeResult};

        let ctx = JSContext::new();
        let class = JSClass::builder("TakeObjTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(99));
        let object = object.as_object().unwrap();

        // Wrong type — data preserved
        assert_eq!(
            object.take_private_data::<String>(),
            PrivateDataTakeResult::TypeMismatch
        );
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 99);

        // Correct type — data taken
        let taken = object.take_private_data::<i32>().unwrap();
        assert_eq!(taken, 99);
        assert!(object.get_private_data::<i32>().is_none());
    }

    #[test]
    fn test_object_drop_private_data_status_preserves_wrong_type() {
        use crate::{JSClass, PrivateDataDropStatus};

        let ctx = JSContext::new();
        let class = JSClass::builder("DropStatusObj").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(77));
        let object = object.as_object().unwrap();

        let wrong_type = object.drop_private_data::<String>();
        assert_eq!(wrong_type, PrivateDataDropStatus::TypeMismatch);
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 77);

        let dropped = object.drop_private_data::<i32>();
        assert_eq!(dropped, PrivateDataDropStatus::Dropped);
        assert!(object.get_private_data::<i32>().is_none());

        let empty = object.drop_private_data::<i32>();
        assert_eq!(empty, PrivateDataDropStatus::Empty);
    }

    #[test]
    fn test_object_set_private_data_replaces_existing_data() {
        use crate::{JSClass, PrivateDataSetStatus};

        let ctx = JSContext::new();
        let class = JSClass::builder("SetReplaceObj").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(1));
        let object = object.as_object().unwrap();

        // SAFETY: the test object was created from `class.build::<i32>()`, so
        // replacing its private data with another `i32` matches the finalizer.
        let status = unsafe { object.set_private_data(2) };
        assert_eq!(status, PrivateDataSetStatus::Replaced);
        assert_eq!(*object.get_private_data::<i32>().unwrap(), 2);
    }

    #[test]
    fn test_object_set_private_data_reports_unsupported_plain_object() {
        use crate::PrivateDataSetStatus;

        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);

        // SAFETY: this intentionally exercises the raw API on a plain object;
        // the method must reject storage and drop the Rust allocation.
        let status = unsafe { object.set_private_data(123i32) };
        assert_eq!(status, PrivateDataSetStatus::Unsupported);
        assert!(object.get_private_data::<i32>().is_none());
    }

    #[test]
    fn test_object_private_data_mut_type_safe() {
        use crate::JSClass;

        let ctx = JSContext::new();
        let class = JSClass::builder("MutObjTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(10));
        let object = object.as_object().unwrap();

        // Wrong type
        assert!(object.get_private_data_mut::<String>().is_none());

        // Correct type
        {
            let mut data = object.get_private_data_mut::<i32>().unwrap();
            *data = 42;
        }

        assert_eq!(*object.get_private_data::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_object_private_data_multiple_immutable_reads() {
        use crate::JSClass;

        let ctx = JSContext::new();
        let class = JSClass::builder("MultiRead").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("stable")));
        let object = object.as_object().unwrap();

        // Multiple shared reads are safe
        let r1 = object.get_private_data::<String>().unwrap();
        let r2 = object.get_private_data::<String>().unwrap();
        assert_eq!(r1, "stable");
        assert_eq!(r2, "stable");
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_object_private_data_guards_block_take_drop_replace_and_mut_aliasing() {
        use crate::{
            JSClass, PrivateDataDropStatus, PrivateDataSetStatus, PrivateDataTakeResult,
        };

        let ctx = JSContext::new();
        let class = JSClass::builder("BorrowGuardObj").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(10));
        let object = object.as_object().unwrap();

        let shared = object.get_private_data::<i32>().unwrap();
        assert!(object.get_private_data_mut::<i32>().is_none());
        assert_eq!(
            object.take_private_data::<i32>(),
            PrivateDataTakeResult::Borrowed
        );
        assert_eq!(
            object.drop_private_data::<i32>(),
            PrivateDataDropStatus::Borrowed
        );
        assert_eq!(
            // SAFETY: this intentionally exercises replacement while a borrow
            // is active; the method must refuse to replace the stored `i32`.
            unsafe { object.set_private_data(20i32) },
            PrivateDataSetStatus::Borrowed
        );
        assert_eq!(*shared, 10);
        drop(shared);

        {
            let mut data = object.get_private_data_mut::<i32>().unwrap();
            assert!(object.get_private_data::<i32>().is_none());
            assert_eq!(
                object.take_private_data::<i32>(),
                PrivateDataTakeResult::Borrowed
            );
            *data = 15;
        }

        assert_eq!(*object.get_private_data::<i32>().unwrap(), 15);
    }

    #[test]
    fn test_object_private_data_refcell_pattern() {
        use crate::JSClass;
        use std::cell::RefCell;

        let ctx = JSContext::new();
        let class = JSClass::builder("RefCellObj")
            .build::<RefCell<i32>>()
            .unwrap();

        let object = class.object::<RefCell<i32>>(&ctx, Some(RefCell::new(0)));
        let object = object.as_object().unwrap();

        // Safe mutation via RefCell
        let cell = object.get_private_data::<RefCell<i32>>().unwrap();
        *cell.borrow_mut() = 100;

        let cell = object.get_private_data::<RefCell<i32>>().unwrap();
        assert_eq!(*cell.borrow(), 100);
    }

    #[test]
    fn test_object_private_data_get_ptr() {
        use crate::JSClass;

        let ctx = JSContext::new();
        let class = JSClass::builder("PtrTest").build::<i32>().unwrap();

        let object = class.object::<i32>(&ctx, Some(42));
        let object = object.as_object().unwrap();

        // Raw pointer access
        let ptr = object.get_private_data_ptr();
        assert!(ptr.is_some());

        // Plain object has no private data pointer
        let plain = JSObject::new(&ctx);
        assert!(plain.get_private_data_ptr().is_none());
    }

    #[test]
    fn test_object_private_data_uaf_scenario() {
        use crate::JSClass;

        let ctx = JSContext::new();
        let class = JSClass::builder("UAFTest").build::<String>().unwrap();

        let object = class.object::<String>(&ctx, Some(String::from("alive")));
        let object = object.as_object().unwrap();
        let object_alias = object.clone();

        // 1. Get reference from first object handle
        let data_ref = object.get_private_data::<String>().unwrap();
        assert_eq!(data_ref, "alive");

        // 2. Taking ownership from an alias now observes the active guard and
        // leaves the allocation in place.
        let taken = object_alias.take_private_data::<String>();
        assert_eq!(taken, crate::PrivateDataTakeResult::Borrowed);
        assert_eq!(data_ref.as_str(), "alive");
        drop(data_ref);

        // Once the guard is dropped, taking ownership succeeds.
        let taken = object_alias.take_private_data::<String>().unwrap();
        assert_eq!(taken, "alive");
        assert!(object.get_private_data::<String>().is_none());
    }
}
