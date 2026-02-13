use std::{
    ffi::{CString, NulError},
    fmt::Debug,
};

use rust_jsc_sys::{
    JSStringCreateWithUTF8CString, JSStringGetLength, JSStringGetMaximumUTF8CStringSize,
    JSStringGetUTF8CString, JSStringIsEqual, JSStringIsEqualToUTF8CString, JSStringRef,
    JSStringRelease,
};

use crate::{JSString, JSStringProctected};

impl JSStringProctected {
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

impl From<&str> for JSStringProctected {
    fn from(s: &str) -> Self {
        let c = CString::new(s.as_bytes())
            .expect("&str to JSStringProctected conversion failed");
        Self(unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) })
    }
}

impl From<String> for JSStringProctected {
    fn from(s: String) -> Self {
        let c = CString::new(s.as_bytes())
            .expect("String to JSStringProctected conversion failed");
        Self(unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) })
    }
}

impl From<JSStringRef> for JSStringProctected {
    fn from(inner: JSStringRef) -> Self {
        Self(inner)
    }
}

impl From<JSStringProctected> for JSStringRef {
    fn from(s: JSStringProctected) -> Self {
        s.0
    }
}

impl std::fmt::Display for JSStringProctected {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(self.0) };
        let mut buffer = vec![0u8; max_len];
        let new_size = unsafe {
            JSStringGetUTF8CString(self.0, buffer.as_mut_ptr() as *mut i8, max_len)
        };
        unsafe {
            buffer.set_len(new_size - 1);
        };
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
        write!(fmt, "{}", s)
    }
}

impl Clone for JSStringProctected {
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
        let utf8 =
            CString::new(other.as_bytes()).expect("JSString to &str conversion failed");
        unsafe { JSStringIsEqualToUTF8CString(self.inner, utf8.as_ptr()) }
    }
}

impl PartialEq<String> for JSString {
    fn eq(&self, other: &String) -> bool {
        let utf8 =
            CString::new(other.as_bytes()).expect("String to JSString conversion failed");
        unsafe { JSStringIsEqualToUTF8CString(self.inner, utf8.as_ptr()) }
    }
}

impl<'s> PartialEq<JSString> for &'s str {
    fn eq(&self, other: &JSString) -> bool {
        let utf8 =
            CString::new(self.as_bytes()).expect("JSString to &str conversion failed");
        unsafe { JSStringIsEqualToUTF8CString(other.inner, utf8.as_ptr()) }
    }
}

impl PartialEq<JSString> for String {
    fn eq(&self, other: &JSString) -> bool {
        let utf8 =
            CString::new(self.as_bytes()).expect("JSString to String conversion failed");
        unsafe { JSStringIsEqualToUTF8CString(other.inner, utf8.as_ptr()) }
    }
}

impl From<&str> for JSString {
    fn from(s: &str) -> Self {
        let c = CString::new(s.as_bytes()).expect("&str to JSString conversion failed");
        JSString {
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        }
    }
}

impl<'a> Into<Vec<u8>> for JSString {
    fn into(self) -> Vec<u8> {
        let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(self.inner) };
        let mut buffer = vec![0u8; max_len];
        let new_size = unsafe {
            JSStringGetUTF8CString(self.inner, buffer.as_mut_ptr() as *mut i8, max_len)
        };
        unsafe {
            buffer.set_len(new_size - 1);
        };

        return buffer;
    }
}

impl TryFrom<&[u8]> for JSString {
    type Error = NulError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        let c = CString::new(s)?;
        Ok(JSString {
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        })
    }
}

impl TryFrom<&mut [u8]> for JSString {
    type Error = NulError;

    fn try_from(s: &mut [u8]) -> Result<Self, Self::Error> {
        let c = CString::new(s)?;
        Ok(JSString {
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        })
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for JSString {
    type Error = NulError;

    fn try_from(s: &[u8; N]) -> Result<Self, Self::Error> {
        JSString::try_from(&s[..])
    }
}

impl From<String> for JSString {
    fn from(s: String) -> Self {
        let c = CString::new(s.as_bytes()).expect("String to JSString conversion failed");
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
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
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
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
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
    use crate::{JSString, JSStringProctected};

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
        let s1 = JSStringProctected::from("Hello, World!");
        let s2 = JSStringProctected::from("Hello, World!");
        let s3 = JSStringProctected::from("démonstration.html");
        let s4 = JSStringProctected::from("こんにちは世界");
        let s5 = JSStringProctected::from("Привет, мир!");
        let s6 = JSStringProctected::from("😊👍🏽");
        let s7 = JSStringProctected::from("");
        let s8 = JSStringProctected::from("你好，世界！");
        let s9 = JSStringProctected::from("Bonjour le monde!");

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
    fn test_js_string_from_bytes() {
        let s = JSString::try_from(b"Hello, World!").unwrap();
        assert_eq!(s.to_string(), "Hello, World!");

        let s = JSString::try_from(&b"Hello, World!"[..]).unwrap();
        assert_eq!(s.to_string(), "Hello, World!");

        let mut data = b"Hello, World!".clone();
        let s = JSString::try_from(&mut data[..]).unwrap();
        assert_eq!(s.to_string(), "Hello, World!");

        // "\uFFFD\uFFFD\uFFFD" in bytes
        let bytes = &[0xEF, 0xBF, 0xBD, 0xEF, 0xBF, 0xBD, 0xEF, 0xBF, 0xBD];
        let s = JSString::try_from(bytes).unwrap();
        assert_eq!(s.to_string(), "\u{FFFD}\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn test_js_string_into_bytes() {
        let s = JSString::try_from(b"Hello, World!").unwrap();
        let bytes: Vec<u8> = s.into();
        assert_eq!(bytes, b"Hello, World!");
    }

    #[test]
    fn test_jsstring_retain() {
        let s = JSStringProctected::from("Hello, World!");
        assert_eq!(s.to_string(), "Hello, World!");

        let s1 = JSStringProctected::from("Hello, World!");
        let s2 = JSStringProctected::from("Hello, World!");
        assert_eq!(s1.clone().to_string(), s2.to_string());
        assert_eq!(s1.to_string(), s2.clone().to_string());
    }
}
