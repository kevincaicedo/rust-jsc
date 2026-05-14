use crate::{
    JSArray, JSContext, JSError, JSFunction, JSObject, JSResult, JSString,
    JSStringProtected, JSTypedArray, JSValue,
};
use std::ops::Deref;

/// Typed rest arguments extracted by callback and constructor macros.
///
/// `Rest<T>` owns the converted arguments after the position where it appears
/// in a macro signature. It must be the last user argument in the signature.
#[derive(Debug, Clone, PartialEq)]
pub struct Rest<T> {
    values: Vec<T>,
}

impl<T> Rest<T> {
    /// Build a rest-argument wrapper from converted values.
    ///
    /// This constructor is public because macro expansion happens in user
    /// crates. Normal users usually receive `Rest<T>` from a macro rather than
    /// constructing it directly.
    pub fn from_vec(values: Vec<T>) -> Self {
        Self { values }
    }

    /// Return the converted rest values as a slice.
    pub fn as_slice(&self) -> &[T] {
        &self.values
    }

    /// Return the number of converted rest values.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return `true` when there are no rest values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Consume the wrapper and return the converted values.
    pub fn into_vec(self) -> Vec<T> {
        self.values
    }
}

impl<T> Deref for Rest<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> IntoIterator for Rest<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Rest<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

/// Fallible conversion from a JavaScript value into a Rust type.
///
/// This is the conversion trait used by typed callback macros. Implementations
/// must preserve JavaScriptCore exception behavior and return a [`JSError`]
/// instead of panicking when conversion fails.
pub trait TryFromJSValue: Sized {
    /// Convert `value` into `Self`.
    ///
    /// # Errors
    /// Returns a JavaScript error when the value cannot be represented by the
    /// requested Rust type or JavaScriptCore throws during conversion.
    fn try_from_js_value(value: &JSValue) -> JSResult<Self>;
}

/// Infallible conversion from a JavaScript value into a Rust type.
///
/// Keep this trait narrow. Most JavaScript conversions can throw or lose
/// information and should implement [`TryFromJSValue`] instead.
pub trait FromJSValue: Sized {
    /// Convert `value` into `Self`.
    fn from_js_value(value: &JSValue) -> Self;
}

/// Fallible conversion from a Rust value into a JavaScript value.
pub trait IntoJSValue {
    /// Convert `self` into a JavaScript value in `ctx`.
    ///
    /// # Errors
    /// Returns a JavaScript error when JavaScriptCore rejects value creation.
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue>;
}

/// Convert a callback return value into a JavaScript callback result.
///
/// This trait is intentionally small today. It gives the macro layer a stable
/// extension point for typed returns without adding hidden allocation or
/// exception behavior.
pub trait IntoJSResult {
    /// Convert `self` into a JavaScript callback result in `ctx`.
    ///
    /// # Errors
    /// Returns a JavaScript error when conversion fails.
    fn into_js_result(self, ctx: &JSContext) -> JSResult<JSValue>;
}

fn value_context(value: &JSValue) -> JSContext {
    // SAFETY: `JSValue` carries the JavaScriptCore context that created the raw
    // value. This borrowed view does not retain or release the context and is
    // used only while reporting a conversion error.
    unsafe { JSContext::borrowed(value.ctx) }
}

fn type_error(value: &JSValue, message: impl Into<JSString>) -> JSError {
    let ctx = value_context(value);
    match JSError::new_typ(&ctx, message) {
        Ok(error) | Err(error) => error,
    }
}

fn checked_integer<T>(value: &JSValue, type_name: &str, min: f64, max: f64) -> JSResult<T>
where
    T: TryFrom<i128>,
{
    let number = value.as_number()?;
    if !number.is_finite() || number.fract() != 0.0 || number < min || number > max {
        return Err(type_error(
            value,
            format!("value cannot be represented as {type_name}"),
        ));
    }

    T::try_from(number as i128).map_err(|_| {
        type_error(value, format!("value cannot be represented as {type_name}"))
    })
}

impl FromJSValue for JSValue {
    fn from_js_value(value: &JSValue) -> Self {
        value.clone()
    }
}

impl TryFromJSValue for JSValue {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        Ok(value.clone())
    }
}

impl FromJSValue for bool {
    fn from_js_value(value: &JSValue) -> Self {
        value.as_boolean()
    }
}

impl TryFromJSValue for bool {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        Ok(value.as_boolean())
    }
}

impl TryFromJSValue for f64 {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        value.as_number()
    }
}

impl TryFromJSValue for i32 {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        checked_integer(value, "i32", i32::MIN as f64, i32::MAX as f64)
    }
}

impl TryFromJSValue for u32 {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        checked_integer(value, "u32", 0.0, u32::MAX as f64)
    }
}

impl TryFromJSValue for usize {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        checked_integer(value, "usize", 0.0, usize::MAX as f64)
    }
}

impl TryFromJSValue for JSString {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        value.as_string()
    }
}

impl TryFromJSValue for String {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        value.as_string().map(|string| string.to_string())
    }
}

impl TryFromJSValue for JSObject {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        value.as_object()
    }
}

impl TryFromJSValue for JSArray {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        if !value.is_array() {
            return Err(type_error(value, "value is not a JavaScript Array"));
        }

        value.as_object().map(JSArray::new)
    }
}

impl TryFromJSValue for JSFunction {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        let object = value.as_object()?;
        if !object.is_function() {
            return Err(type_error(value, "value is not a JavaScript function"));
        }

        Ok(JSFunction::from(object))
    }
}

impl TryFromJSValue for JSTypedArray {
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        JSTypedArray::from_value(value)
    }
}

impl<T> TryFromJSValue for Option<T>
where
    T: TryFromJSValue,
{
    fn try_from_js_value(value: &JSValue) -> JSResult<Self> {
        if value.is_undefined() || value.is_null() {
            Ok(None)
        } else {
            T::try_from_js_value(value).map(Some)
        }
    }
}

impl IntoJSValue for JSValue {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self)
    }
}

impl IntoJSValue for &JSValue {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self.clone())
    }
}

impl IntoJSValue for bool {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::boolean(ctx, self))
    }
}

impl IntoJSValue for f64 {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::number(ctx, self))
    }
}

impl IntoJSValue for i32 {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::number(ctx, self as f64))
    }
}

impl IntoJSValue for u32 {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::number(ctx, self as f64))
    }
}

impl IntoJSValue for usize {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::number(ctx, self as f64))
    }
}

impl IntoJSValue for &str {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::string(ctx, self))
    }
}

impl IntoJSValue for String {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::string(ctx, self))
    }
}

impl IntoJSValue for JSString {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::string(ctx, self))
    }
}

impl IntoJSValue for JSStringProtected {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::string_retain(ctx, self))
    }
}

impl IntoJSValue for JSObject {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self.into())
    }
}

impl IntoJSValue for JSArray {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self.into())
    }
}

impl IntoJSValue for JSFunction {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self.into())
    }
}

impl IntoJSValue for JSTypedArray {
    fn into_js_value(self, _ctx: &JSContext) -> JSResult<JSValue> {
        Ok(self.into())
    }
}

impl<T> IntoJSValue for Option<T>
where
    T: IntoJSValue,
{
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        match self {
            Some(value) => value.into_js_value(ctx),
            None => Ok(JSValue::undefined(ctx)),
        }
    }
}

impl IntoJSValue for () {
    fn into_js_value(self, ctx: &JSContext) -> JSResult<JSValue> {
        Ok(JSValue::undefined(ctx))
    }
}

impl<T> IntoJSResult for T
where
    T: IntoJSValue,
{
    fn into_js_result(self, ctx: &JSContext) -> JSResult<JSValue> {
        self.into_js_value(ctx)
    }
}

impl<T> IntoJSResult for JSResult<T>
where
    T: IntoJSValue,
{
    fn into_js_result(self, ctx: &JSContext) -> JSResult<JSValue> {
        self.and_then(|value| value.into_js_value(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::{IntoJSResult, IntoJSValue, TryFromJSValue};
    use crate::{JSContext, JSError, JSResult, JSString, JSValue};

    #[test]
    fn option_conversion_treats_missing_js_values_as_none() {
        let ctx = JSContext::new();

        let undefined = JSValue::undefined(&ctx);
        let null = JSValue::null(&ctx);
        let string = JSValue::string(&ctx, "value");

        assert_eq!(
            Option::<JSString>::try_from_js_value(&undefined)
                .unwrap()
                .map(|value| value.to_string()),
            None
        );
        assert_eq!(
            Option::<JSString>::try_from_js_value(&null)
                .unwrap()
                .map(|value| value.to_string()),
            None
        );
        assert_eq!(
            Option::<JSString>::try_from_js_value(&string)
                .unwrap()
                .map(|value| value.to_string()),
            Some("value".to_string())
        );
    }

    #[test]
    fn integer_conversion_rejects_fractional_and_out_of_range_numbers() {
        let ctx = JSContext::new();
        let fractional = JSValue::number(&ctx, 1.5);
        let out_of_range = JSValue::number(&ctx, -1.0);
        let valid = JSValue::number(&ctx, 42.0);

        assert!(u32::try_from_js_value(&fractional).is_err());
        assert!(u32::try_from_js_value(&out_of_range).is_err());
        assert_eq!(u32::try_from_js_value(&valid).unwrap(), 42);
    }

    #[test]
    fn rust_values_convert_into_js_values_with_context() {
        let ctx = JSContext::new();

        assert!(true.into_js_value(&ctx).unwrap().as_boolean());
        assert_eq!(7u32.into_js_value(&ctx).unwrap().as_number().unwrap(), 7.0);
        assert_eq!(
            "hello".into_js_value(&ctx).unwrap().as_string().unwrap(),
            "hello"
        );
        assert!(Option::<u32>::None
            .into_js_value(&ctx)
            .unwrap()
            .is_undefined());
    }

    #[test]
    fn callback_returns_convert_through_into_js_result() {
        let ctx = JSContext::new();

        assert_eq!(
            "hello".into_js_result(&ctx).unwrap().as_string().unwrap(),
            "hello"
        );
        assert!(JSResult::Ok(true)
            .into_js_result(&ctx)
            .unwrap()
            .as_boolean());
        assert!(().into_js_result(&ctx).unwrap().is_undefined());

        let err: JSResult<bool> = Err(JSError::new_typ(&ctx, "boom").unwrap());
        assert!(err.into_js_result(&ctx).is_err());
    }
}
