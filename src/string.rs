use std::{ffi::CString, fmt::Debug};

use rust_jsc_sys::{
    JSStringCreateWithUTF8CString, JSStringGetLength, JSStringGetMaximumUTF8CStringSize,
    JSStringGetUTF8CString, JSStringIsEqual, JSStringIsEqualToUTF8CString, JSStringRef,
    JSStringRelease,
};

use crate::{JSString, JSStringRetain};

impl JSStringRetain {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        unsafe { JSStringGetLength(self.0) }
    }

    pub fn release(&self) {
        unsafe {
            JSStringRelease(self.0);
        }
    }
}

impl From<&str> for JSStringRetain {
    fn from(s: &str) -> Self {
        let c = CString::new(s.as_bytes()).unwrap();
        Self(unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) })
    }
}

impl From<String> for JSStringRetain {
    fn from(s: String) -> Self {
        let c = CString::new(s.as_bytes()).unwrap();
        Self(unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) })
    }
}

impl From<JSStringRef> for JSStringRetain {
    fn from(inner: JSStringRef) -> Self {
        Self(inner)
    }
}

impl From<JSStringRetain> for JSStringRef {
    fn from(s: JSStringRetain) -> Self {
        s.0
    }
}

impl std::fmt::Display for JSStringRetain {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(self.0) };
        let mut buffer = vec![0u8; max_len];
        let new_size = unsafe {
            JSStringGetUTF8CString(self.0, buffer.as_mut_ptr() as *mut i8, max_len)
        };
        unsafe {
            buffer.set_len(new_size - 1);
        };
        let s = String::from_utf8(buffer).unwrap();
        write!(fmt, "{}", s)
    }
}

impl Clone for JSStringRetain {
    fn clone(&self) -> Self {
        self.to_string().into()
    }
}

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

impl Clone for JSString {
    fn clone(&self) -> Self {
        self.to_string().into()
    }
}

impl Debug for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(self.inner) };
        let mut buffer = vec![0u8; max_len];
        let new_size = unsafe {
            JSStringGetUTF8CString(self.inner, buffer.as_mut_ptr() as *mut i8, max_len)
        };
        unsafe {
            buffer.set_len(new_size - 1);
        };
        let s = String::from_utf8(buffer).unwrap();
        write!(fmt, "{:?}", s)
    }
}

impl std::fmt::Display for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(self.inner) };
        let mut buffer = vec![0u8; max_len];
        let new_size = unsafe {
            JSStringGetUTF8CString(self.inner, buffer.as_mut_ptr() as *mut i8, max_len)
        };
        unsafe {
            buffer.set_len(new_size - 1);
        };
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

#[cfg(test)]
mod tests {
    use crate::{JSString, JSStringRetain};

    #[test]
    fn test_js_string() {
        let s = JSString::from("Hello, World!");
        assert_eq!(s.len(), 13);
        assert_eq!(s.to_string(), "Hello, World!");
    }

    #[test]
    fn test_js_string_eq() {
        let s1 = JSString::from("Hello, World!");
        let s2 = JSString::from("Hello, World!");
        let s3 = JSString::from("démonstration.html");
        assert_eq!(s1, s2);
        assert_eq!(s1, "Hello, World!");
        assert_eq!(s2, "Hello, World!");
        assert_eq!(s3.to_string(), "démonstration.html");
        assert_eq!("Hello, World!", s1);
        assert_eq!("Hello, World!", s2);
        assert_eq!("démonstration.html", s3);
    }

    #[test]
    fn test_js_string_retain_eq_utf8() {
        let s1 = JSStringRetain::from("Hello, World!");
        let s2 = JSStringRetain::from("Hello, World!");
        let s3 = JSStringRetain::from("démonstration.html");
        let s4 = JSStringRetain::from("こんにちは世界");
        let s5 = JSStringRetain::from("Привет, мир!");
        let s6 = JSStringRetain::from("😊👍🏽");
        let s7 = JSStringRetain::from("");
        let s8 = JSStringRetain::from("你好，世界！");
        let s9 = JSStringRetain::from("Bonjour le monde!");

        // Test equality with the same content
        assert_eq!(s1.to_string(), s2.to_string());

        // Test special characters and different languages
        assert_eq!(s3.to_string(), "démonstration.html");
        assert_eq!(s4.to_string(), "こんにちは世界");
        assert_eq!(s5.to_string(), "Привет, мир!");
        assert_eq!(s6.to_string(), "😊👍🏽");
        assert_eq!(s8.to_string(), "你好，世界！");
        assert_eq!(s9.to_string(), "Bonjour le monde!");

        // Test empty string
        assert!(s7.is_empty());
        assert_eq!(s7.len(), 0);
    }

    #[test]
    fn test_js_string_eq_utf8() {
        let s1 = JSString::from("Hello, World!");
        let s2 = JSString::from("Hello, World!");
        let s3 = JSString::from("démonstration.html");
        let s4 = JSString::from("こんにちは世界");
        let s5 = JSString::from("Привет, мир!");
        let s6 = JSString::from("😊👍🏽");
        let s7 = JSString::from("");
        let s8 = JSString::from("你好，世界！");
        let s9 = JSString::from("Bonjour le monde!");

        // Test equality with the same content
        assert_eq!(s1, s2);
        assert_eq!(s1, "Hello, World!");
        assert_eq!(s2, "Hello, World!");

        // Test special characters and different languages
        assert_eq!(s3.to_string(), "démonstration.html");
        assert_eq!(s4.to_string(), "こんにちは世界");
        assert_eq!(s5.to_string(), "Привет, мир!");
        assert_eq!(s6.to_string(), "😊👍🏽");
        assert_eq!(s8.to_string(), "你好，世界！");
        assert_eq!(s9.to_string(), "Bonjour le monde!");

        // Test empty string
        assert!(s7.is_empty());
        assert_eq!(s7.len(), 0);

        // Test reverse equality with &str and String
        assert_eq!("Hello, World!", s1);
        assert_eq!("Hello, World!", s2);
        assert_eq!("démonstration.html", s3);
        assert_eq!("こんにちは世界", s4);
        assert_eq!("Привет, мир!", s5);
        assert_eq!("😊👍🏽", s6);
        assert_eq!("", s7);
        assert_eq!("你好，世界！", s8);
        assert_eq!("Bonjour le monde!", s9);
    }

    #[test]
    fn test_js_string_debug() {
        let js_string = JSString::from("debug test");
        assert_eq!(format!("{:?}", js_string), r#""debug test""#);
    }

    #[test]
    fn test_js_string_display() {
        let js_string = JSString::from("display test");
        assert_eq!(format!("{}", js_string), "display test");
    }

    #[test]
    fn test_js_string_len() {
        let s = JSString::from("Hello, World!");
        assert_eq!(s.len(), 13);
    }

    #[test]
    fn test_js_string_is_empty() {
        let s = JSString::from("");
        assert_eq!(s.is_empty(), true);
    }

    #[test]
    fn test_jsstring_retain() {
        let s = JSStringRetain::from("Hello, World!");
        assert_eq!(s.to_string(), "Hello, World!");

        let s1 = JSStringRetain::from("Hello, World!");
        let s2 = JSStringRetain::from("Hello, World!");
        assert_eq!(s1.clone().to_string(), s2.to_string());
        assert_eq!(s1.to_string(), s2.clone().to_string());
    }
}
