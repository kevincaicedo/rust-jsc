use std::{
    ffi::{CString, NulError},
    fmt::Debug,
    os::raw::c_char,
};

use rust_jsc_sys::{
    JSChar, JSStringCreateWithCharacters, JSStringCreateWithUTF8CString,
    JSStringGetLength, JSStringGetMaximumUTF8CStringSize, JSStringGetUTF8CString,
    JSStringIsEqual, JSStringIsEqualToUTF8CString, JSStringRef, JSStringRelease,
    JSStringRetain,
};

use crate::{JSString, JSStringProtected};

fn create_js_string_ref(value: &str) -> JSStringRef {
    match CString::new(value.as_bytes()) {
        Ok(value) => {
            // SAFETY: `CString` provides a live NUL-terminated UTF-8 buffer for
            // the duration of the call, and JavaScriptCore copies the input.
            unsafe { JSStringCreateWithUTF8CString(value.as_ptr()) }
        }
        Err(_) => {
            let utf16: Vec<JSChar> = value.encode_utf16().collect();
            // SAFETY: `utf16.as_ptr()` points to `utf16.len()` initialized
            // UTF-16 code units for the duration of the call, and
            // JavaScriptCore copies the input.
            unsafe { JSStringCreateWithCharacters(utf16.as_ptr(), utf16.len()) }
        }
    }
}

fn js_string_ref_to_utf8_bytes(inner: JSStringRef) -> Vec<u8> {
    // SAFETY: callers pass a live `JSStringRef`; this only asks
    // JavaScriptCore for the maximum UTF-8 buffer size.
    let max_len = unsafe { JSStringGetMaximumUTF8CStringSize(inner) };
    if max_len == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; max_len];
    // SAFETY: `buffer` is initialized with `max_len` bytes and the pointer is
    // writable for exactly that size. JavaScriptCore writes a NUL-terminated
    // UTF-8 string and returns the byte count including the terminator.
    let new_size = unsafe {
        JSStringGetUTF8CString(inner, buffer.as_mut_ptr() as *mut c_char, max_len)
    };
    buffer.truncate(new_size.saturating_sub(1));
    buffer
}

fn js_string_ref_eq_str(inner: JSStringRef, other: &str) -> bool {
    match CString::new(other.as_bytes()) {
        Ok(other) => {
            // SAFETY: `inner` is a live `JSStringRef`, and `other` is a
            // NUL-terminated UTF-8 buffer for the duration of the call.
            unsafe { JSStringIsEqualToUTF8CString(inner, other.as_ptr()) }
        }
        Err(_) => {
            let other = JSString::from(other);
            // SAFETY: both inputs are live JavaScriptCore strings.
            unsafe { JSStringIsEqual(inner, other.inner) }
        }
    }
}

impl JSStringProtected {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        // SAFETY: `self.0` is a live `JSStringRef` retained by this wrapper.
        unsafe { JSStringGetLength(self.0) }
    }

    /// Releases this retained string before the end of its scope.
    pub fn release(self) {
        drop(self);
    }

    /// Transfers the retained string reference to another owner.
    ///
    /// The caller becomes responsible for exactly one `JSStringRelease` or
    /// equivalent JavaScriptCore deref on the returned reference.
    pub fn into_raw(self) -> JSStringRef {
        let inner = self.0;
        std::mem::forget(self);
        inner
    }

    /// Takes ownership of a retained JavaScriptCore string reference.
    ///
    /// # Safety
    /// `inner` must be a non-null `JSStringRef` owned by the caller. After this
    /// call, this wrapper releases it exactly once unless ownership is moved
    /// with [`JSStringProtected::into_raw`].
    pub unsafe fn from_owned_ref(inner: JSStringRef) -> Self {
        Self(inner)
    }
}

impl From<&str> for JSStringProtected {
    fn from(s: &str) -> Self {
        Self(create_js_string_ref(s))
    }
}

impl From<String> for JSStringProtected {
    fn from(s: String) -> Self {
        Self(create_js_string_ref(&s))
    }
}

impl From<JSStringProtected> for JSStringRef {
    fn from(s: JSStringProtected) -> Self {
        s.into_raw()
    }
}

impl std::fmt::Display for JSStringProtected {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let buffer = js_string_ref_to_utf8_bytes(self.0);
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
        write!(fmt, "{}", s)
    }
}

impl Clone for JSStringProtected {
    fn clone(&self) -> Self {
        self.to_string().into()
    }
}

impl Drop for JSStringProtected {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is the retained string reference owned by this
            // RAII wrapper and is released exactly once from `Drop`.
            unsafe {
                JSStringRelease(self.0);
            }
        }
    }
}

impl JSString {
    /// Creates a new `JSString` object.
    #[allow(dead_code)]
    pub(crate) fn new(inner: JSStringRef) -> Self {
        Self { inner }
    }

    /// Takes ownership of a JavaScriptCore string reference.
    ///
    /// # Safety
    /// `inner` must be a non-null `JSStringRef` owned by the caller. The
    /// returned wrapper releases it exactly once in `Drop`.
    pub unsafe fn from_owned_ref(inner: JSStringRef) -> Self {
        Self::new(inner)
    }

    /// Retains a borrowed JavaScriptCore string reference.
    ///
    /// # Safety
    /// `inner` must be a non-null live `JSStringRef`. The returned wrapper owns
    /// one retained reference and releases it in `Drop`.
    pub unsafe fn retain_from_ref(inner: JSStringRef) -> Self {
        // SAFETY: caller guarantees `inner` is a live JavaScriptCore string.
        Self::new(unsafe { JSStringRetain(inner) })
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        // SAFETY: `self.inner` is a live `JSStringRef` owned by this wrapper.
        unsafe { JSStringGetLength(self.inner) }
    }
}

impl PartialEq for JSString {
    fn eq(&self, other: &JSString) -> bool {
        // SAFETY: both wrappers hold live JavaScriptCore strings.
        unsafe { JSStringIsEqual(self.inner, other.inner) }
    }
}

impl<'s> PartialEq<&'s str> for JSString {
    fn eq(&self, other: &&'s str) -> bool {
        js_string_ref_eq_str(self.inner, other)
    }
}

impl PartialEq<String> for JSString {
    fn eq(&self, other: &String) -> bool {
        js_string_ref_eq_str(self.inner, other)
    }
}

impl PartialEq<JSString> for &str {
    fn eq(&self, other: &JSString) -> bool {
        js_string_ref_eq_str(other.inner, self)
    }
}

impl PartialEq<JSString> for String {
    fn eq(&self, other: &JSString) -> bool {
        js_string_ref_eq_str(other.inner, self)
    }
}

impl From<&str> for JSString {
    fn from(s: &str) -> Self {
        JSString {
            inner: create_js_string_ref(s),
        }
    }
}

impl From<JSString> for Vec<u8> {
    fn from(value: JSString) -> Self {
        js_string_ref_to_utf8_bytes(value.inner)
    }
}

impl TryFrom<&[u8]> for JSString {
    type Error = NulError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        let c = CString::new(s)?;
        Ok(JSString {
            // SAFETY: `CString` provides a live NUL-terminated UTF-8 buffer for
            // the duration of the call, and JavaScriptCore copies the input.
            inner: unsafe { JSStringCreateWithUTF8CString(c.as_ptr()) },
        })
    }
}

impl TryFrom<&mut [u8]> for JSString {
    type Error = NulError;

    fn try_from(s: &mut [u8]) -> Result<Self, Self::Error> {
        let c = CString::new(s)?;
        Ok(JSString {
            // SAFETY: `CString` provides a live NUL-terminated UTF-8 buffer for
            // the duration of the call, and JavaScriptCore copies the input.
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
        JSString {
            inner: create_js_string_ref(&s),
        }
    }
}

impl Clone for JSString {
    fn clone(&self) -> Self {
        self.to_string().into()
    }
}

impl Debug for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let buffer = js_string_ref_to_utf8_bytes(self.inner);
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
        write!(fmt, "{:?}", s)
    }
}

impl std::fmt::Display for JSString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let buffer = js_string_ref_to_utf8_bytes(self.inner);
        let s = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
        write!(fmt, "{}", s)
    }
}

impl Drop for JSString {
    fn drop(&mut self) {
        // SAFETY: `self.inner` is an owned `JSStringRef` for this wrapper and
        // is released exactly once from `Drop`.
        unsafe {
            JSStringRelease(self.inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{JSString, JSStringProtected};

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
        let s1 = JSStringProtected::from("Hello, World!");
        let s2 = JSStringProtected::from("Hello, World!");
        let s3 = JSStringProtected::from("démonstration.html");
        let s4 = JSStringProtected::from("こんにちは世界");
        let s5 = JSStringProtected::from("Привет, мир!");
        let s6 = JSStringProtected::from("😊👍🏽");
        let s7 = JSStringProtected::from("");
        let s8 = JSStringProtected::from("你好，世界！");
        let s9 = JSStringProtected::from("Bonjour le monde!");

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
    fn test_js_string_interior_nul_does_not_panic_or_truncate() {
        let value = "left\0right";
        let js_string = JSString::from(value);
        assert_eq!(js_string.to_string(), value);
        assert_eq!(js_string, value);
        assert_eq!(value, js_string);

        let protected = JSStringProtected::from(value);
        assert_eq!(protected.to_string(), value);
    }

    #[test]
    fn test_js_string_len() {
        let s = JSString::from("Hello, World!");
        assert_eq!(s.len(), 13);
    }

    #[test]
    fn test_js_string_is_empty() {
        let s = JSString::from("");
        assert!(s.is_empty());
    }

    #[test]
    fn test_js_string_from_bytes() {
        let s = JSString::try_from(b"Hello, World!").unwrap();
        assert_eq!(s.to_string(), "Hello, World!");

        let s = JSString::try_from(&b"Hello, World!"[..]).unwrap();
        assert_eq!(s.to_string(), "Hello, World!");

        let mut data = *b"Hello, World!";
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

        let s = JSString::try_from(b"Hello, World!").unwrap();
        assert_eq!(Vec::<u8>::from(s), b"Hello, World!");
    }

    #[test]
    fn test_jsstring_retain() {
        let s = JSStringProtected::from("Hello, World!");
        assert_eq!(s.to_string(), "Hello, World!");

        let s1 = JSStringProtected::from("Hello, World!");
        let s2 = JSStringProtected::from("Hello, World!");
        assert_eq!(s1.clone().to_string(), s2.to_string());
        assert_eq!(s1.to_string(), s2.clone().to_string());

        JSStringProtected::from("release early").release();
    }
}
