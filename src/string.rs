use std::{ffi::CString, fmt::Debug};

use rust_jsc_sys::{
    JSStringCreateWithUTF8CString, JSStringGetLength, JSStringGetUTF8CString,
    JSStringIsEqual, JSStringIsEqualToUTF8CString, JSStringRef, JSStringRelease,
};

use crate::JSString;

impl JSString {
    /// Creates a new `JSString` object.
    #[allow(dead_code)]
    pub(crate) fn new(inner: JSStringRef) -> Self {
        Self { inner }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        unsafe { JSStringGetLength(self.inner) }
    }
}

impl PartialEq for JSString {
    fn eq(&self, other: &JSString) -> bool {
        unsafe { JSStringIsEqual(self.inner, other.inner) }
    }
}

impl<'s> PartialEq<&'s str> for JSString {
    fn eq(&self, other: &&'s str) -> bool {
        let utf8 = CString::new(other.as_bytes()).unwrap();
        unsafe { JSStringIsEqualToUTF8CString(self.inner, utf8.as_ptr()) }
    }
}

impl PartialEq<String> for JSString {
    fn eq(&self, other: &String) -> bool {
        let utf8 = CString::new(other.as_bytes()).unwrap();
        unsafe { JSStringIsEqualToUTF8CString(self.inner, utf8.as_ptr()) }
    }
}

impl<'s> PartialEq<JSString> for &'s str {
    fn eq(&self, other: &JSString) -> bool {
        let utf8 = CString::new(self.as_bytes()).unwrap();
        unsafe { JSStringIsEqualToUTF8CString(other.inner, utf8.as_ptr()) }
    }
}

impl PartialEq<JSString> for String {
    fn eq(&self, other: &JSString) -> bool {
        let utf8 = CString::new(self.as_bytes()).unwrap();
        unsafe { JSStringIsEqualToUTF8CString(other.inner, utf8.as_ptr()) }
    }
}

impl From<&str> for JSString {
    fn from(s: &str) -> Self {
        let c = CString::new(s.as_bytes()).unwrap();
        JSString {
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        }
    }
}

impl From<String> for JSString {
    fn from(s: String) -> Self {
        let c = CString::new(s.as_bytes()).unwrap();
        JSString {
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        }
    }
}

impl From<JSStringRef> for JSString {
    fn from(inner: JSStringRef) -> Self {
        Self { inner }
    }
}

impl Debug for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let len = self.len();
        let mut buffer = vec![0u8; len + 1];
        unsafe {
            JSStringGetUTF8CString(self.inner, buffer.as_mut_ptr() as *mut i8, len + 1);
        }
        let s = String::from_utf8(buffer).unwrap();
        write!(fmt, "{:?}", s)
    }
}

impl std::fmt::Display for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let len = self.len();
        let mut buffer = vec![0u8; len + 1];
        unsafe {
            JSStringGetUTF8CString(self.inner, buffer.as_mut_ptr() as *mut i8, len + 1);
        }
        let s = String::from_utf8(buffer).unwrap();
        write!(fmt, "{}", s)
    }
}

impl Drop for JSString {
    fn drop(&mut self) {
        unsafe {
            JSStringRelease(self.inner);
        }
    }
}
