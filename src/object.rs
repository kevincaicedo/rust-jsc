use std::ops::Deref;

use rust_jsc_sys::{
    JSContextRef, JSObjectCallAsConstructor, JSObjectCallAsFunction,
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
    JSContext, JSError, JSObject, JSResult, JSString, JSValue, PropertyDescriptor,
};

pub struct JSPropertyNameIter {
    inner: JSPropertyNameArrayRef,
    index: usize,
}

impl Iterator for JSPropertyNameIter {
    type Item = JSString;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < unsafe { JSPropertyNameArrayGetCount(self.inner) } {
            let name =
                unsafe { JSPropertyNameArrayGetNameAtIndex(self.inner, self.index) };
            self.index += 1;
            Some(JSString {
                inner: unsafe { JSStringRetain(name) },
            })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sz = unsafe { JSPropertyNameArrayGetCount(self.inner) };
        (sz - self.index, Some(sz))
    }
}

impl Drop for JSPropertyNameIter {
    fn drop(&mut self) {
        unsafe { JSPropertyNameArrayRelease(self.inner) }
    }
}

impl JSObject {
    /// Creates a new `JSObject` object.
    ///
    /// Creates a new empty JavaScript object.
    pub fn new(ctx: &JSContext) -> Self {
        let inner = unsafe {
            JSObjectMake(ctx.inner, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let value = JSValue::new(inner, ctx.inner);
        Self { inner, value }
    }

    pub fn from_ref(inner: JSObjectRef, ctx: JSContextRef) -> Self {
        let value = JSValue::new(inner, ctx);
        Self { inner, value }
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
        let result = unsafe {
            JSObjectGetPropertyAtIndex(self.value.ctx, self.inner, index, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
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
        let result = unsafe {
            JSObjectGetPropertyForKey(self.ctx, self.inner, key.inner, &mut exception)
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
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
            unsafe { JSObjectGetPrototype(self.value.ctx, self.inner) },
            self.value.ctx,
        )
    }

    /// Sets an object's prototype.
    /// This function is the same as performing "Object.setPrototypeOf(object, prototype)" from JavaScript.
    ///
    /// # Arguments
    /// * `prototype` - The prototype to set on the object.
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
        unsafe {
            JSObjectSetPrototype(self.ctx, self.inner, prototype.inner);
        }
    }

    /// Sets a pointer to private data on an object.
    /// The default object class does not allocate storage for private data.
    /// Only objects created with a non-NULL JSClass can store private data.
    ///
    /// # Arguments
    /// * `data` - The private data to set on the object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let data = Box::new(42);
    /// object.set_private_data(data);
    ///
    /// let private_data: Box<i32> = object.get_private_data().unwrap();
    /// assert_eq!(*private_data, 42);
    /// ```
    ///
    /// # Returns
    /// Returns true if object can store private data, otherwise false.
    pub fn set_private_data<T>(&self, data: Box<T>) -> bool {
        let data_ptr = Box::into_raw(data);
        unsafe { JSObjectSetPrivate(self.inner, data_ptr as _) }
    }

    /// Gets the private data from an object.
    ///
    /// # Example
    /// ```no_run
    /// use rust_jsc::*;
    ///
    /// let ctx = JSContext::new();
    /// let object = JSObject::new(&ctx);
    /// let data = Box::new(42);
    /// object.set_private_data(data);
    ///
    /// let private_data: Box<i32> = object.get_private_data().unwrap();
    /// assert_eq!(*private_data, 42);
    /// ```
    ///
    /// # Returns
    /// Returns the private data if it exists, otherwise None.
    pub fn get_private_data<T>(&self) -> Option<Box<T>> {
        let data_ptr = unsafe { JSObjectGetPrivate(self.inner) };

        if data_ptr.is_null() {
            return None;
        }

        Some(unsafe { Box::from_raw(data_ptr as *mut T) })
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
    /// assert_eq!(object.is_contructor(), false);
    /// ```
    ///
    /// # Returns
    /// Returns true if the object can be called as a constructor, otherwise false.
    pub fn is_contructor(&self) -> bool {
        unsafe { JSObjectIsConstructor(self.value.ctx, self.inner) }
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
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();
        let result = unsafe {
            JSObjectCallAsConstructor(
                self.value.ctx,
                self.inner,
                args.len(),
                args.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(JSObject::from_ref(result, self.value.ctx))
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
        let args: Vec<JSValueRef> = args.iter().map(|arg| arg.inner).collect();
        let this_object = this.map_or(std::ptr::null_mut(), |this| this.inner);
        let result = unsafe {
            JSObjectCallAsFunction(
                self.value.ctx,
                self.inner,
                this_object,
                args.len(),
                args.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            let value = JSValue::new(exception, self.value.ctx);
            return Err(JSError::from(value));
        }

        Ok(JSValue::new(result, self.value.ctx))
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

    use crate::{JSContext, JSFunction, JSObject, JSResult, JSValue, PropertyDescriptor};

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
        assert_eq!(object.has(&key).unwrap(), true);
        assert_eq!(object.delete(&key).unwrap(), true);
        assert_eq!(object.has(&key).unwrap(), false);
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
        assert_eq!(object.has_property(name), true);
        assert_eq!(object.delete_property(name).unwrap(), true);
        assert_eq!(object.has_property(name), false);
    }

    #[test]
    fn test_object_prototype() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let prototype = JSObject::new(&ctx);

        object.set_prototype(&prototype);
        assert_eq!(object.get_prototype(), prototype.into());
    }

    // #[test]
    // fn test_object_private_data() {
    //     let ctx = JSContext::new();
    //     let object = JSObject::new(&ctx);
    //     let data = Box::new(42);

    //     object.set_private_data(data.clone());
    //     let private_data: Box<i32> = object.get_private_data().unwrap();
    //     assert_eq!(*private_data, 42);
    // }

    #[test]
    fn test_object_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let result = object.call_as_constructor(&[]).unwrap();
        assert_eq!(result.is_contructor(), false);
    }

    #[test]
    fn test_object_function() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert_eq!(object.is_function(), false);

        let function = ctx
            .evaluate_script("function test() { return 42; }; test", None)
            .unwrap();
        assert_eq!(function.is_object(), true);

        let function = function.as_object().unwrap();
        assert_eq!(function.is_function(), true);

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
        assert_eq!(property_names.next(), Some(JSString::from("key")));
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
        assert_eq!(object.has_property(name), true);
        assert_eq!(object.delete_property(name).unwrap(), true);
        assert_eq!(object.has_property(name), false);
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
        assert_eq!(object.has_property("name"), true);
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
        assert_eq!(object.has(&key).unwrap(), true);
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
        assert_eq!(object.has(&key).unwrap(), true);
        assert_eq!(object.delete(&key).unwrap(), true);
        assert_eq!(object.has(&key).unwrap(), false);
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

    // #[test]
    // fn test_object_set_private_data() {
    //     let ctx = JSContext::new();
    //     let object = JSObject::new(&ctx);
    //     let data = Box::new(42);

    //     object.set_private_data(data.clone());
    //     let private_data: Box<i32> = object.get_private_data().unwrap();
    //     assert_eq!(*private_data, 42);
    // }

    // #[test]
    // fn test_object_get_private_data() {
    //     let ctx = JSContext::new();
    //     let object = JSObject::new(&ctx);
    //     let data = Box::new(42);

    //     object.set_private_data(data.clone());
    //     let private_data: Box<i32> = object.get_private_data().unwrap();
    //     assert_eq!(*private_data, 42);
    // }

    #[test]
    fn test_object_is_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert_eq!(object.is_contructor(), false);
    }

    #[test]
    fn test_object_is_function() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        assert_eq!(object.is_function(), false);
    }

    #[test]
    fn test_object_call_as_constructor() {
        let ctx = JSContext::new();
        let object = JSObject::new(&ctx);
        let result = object.call_as_constructor(&[]).unwrap();
        assert_eq!(result.is_contructor(), false);
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

        let mut property_names = object.get_property_names();
        assert_eq!(property_names.next(), Some(JSString::from("key")));
        assert_eq!(property_names.next(), None);
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
            let message = arguments.get(0).unwrap().as_string().unwrap();
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

        assert_eq!(result.is_ok(), true);
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

        assert_eq!(result.is_ok(), true);
    }
}
