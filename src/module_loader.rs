use std::{
    ffi::CString,
    fmt,
    path::{Path, PathBuf},
};

use rust_jsc_sys::{
    JSAPIModuleLoader, JSContextRef, JSModuleLoaderCreateImportMetaProperties,
    JSModuleLoaderEvaluate, JSModuleLoaderFetch, JSModuleLoaderFetchSource,
    JSModuleLoaderResolve, JSModuleSourceCreateJSON, JSModuleSourceCreateJavaScript,
    JSModuleSourceCreateWebAssembly, JSModuleSourceRef, JSModuleSourceRelease,
    JSObjectRef, JSStringCreateWithUTF8CString, JSStringRef, JSValueRef,
};

use crate::{
    not_send_or_sync, JSContext, JSError, JSObject, JSResult, JSString,
    JSStringProtected, JSValue, NotSendOrSync,
};

/// Import type requested by JavaScriptCore for a module fetch.
///
/// WebKit passes `"json"` for imports such as
/// `import data from "./data.json" with { type: "json" }`. The default file
/// loader uses this to return raw JSON for JSON-module parsing while keeping
/// legacy bare `.json` imports as JavaScript modules with a default export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleImportType {
    Unknown,
    JavaScript,
    Json,
    WebAssembly,
}

impl ModuleImportType {
    /// Convert a JavaScript import-attributes value into the typed import kind.
    ///
    /// JavaScriptCore currently passes a string such as `"json"`,
    /// `"javascript"`, or `"webassembly"` for typed imports. `undefined`,
    /// `null`, non-string values, and unknown strings map to
    /// [`ModuleImportType::Unknown`].
    pub fn from_js_value(value: &JSValue) -> Self {
        if value.is_undefined() || value.is_null() {
            return Self::Unknown;
        }

        let Ok(value) = value.as_string() else {
            return Self::Unknown;
        };

        match value.to_string().as_str() {
            "javascript" => Self::JavaScript,
            "json" => Self::Json,
            "webassembly" => Self::WebAssembly,
            _ => Self::Unknown,
        }
    }

    fn from_attributes_value(ctx: JSContextRef, attributes_value: JSValueRef) -> Self {
        if ctx.is_null() || attributes_value.is_null() {
            return Self::Unknown;
        }

        let value = JSValue::new(attributes_value, ctx);
        Self::from_js_value(&value)
    }
}

/// Typed module source returned by safe Rust loader helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSource {
    JavaScript(String),
    Json(String),
    WebAssembly(Vec<u8>),
}

impl ModuleSource {
    fn into_js_module_source(self) -> Option<JSModuleSource> {
        match self {
            Self::JavaScript(source) => JSModuleSource::javascript(&source),
            Self::Json(source) => JSModuleSource::json(&source),
            Self::WebAssembly(bytes) => JSModuleSource::webassembly(&bytes),
        }
    }
}

/// Structured error for resolver, fetcher, and import-meta failures.
///
/// Use this in custom module loaders when diagnostics should include the
/// module id and optional referrer without hard-coding runtime policy into
/// `rust-jsc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLoadError {
    module_id: String,
    referrer: Option<String>,
    message: String,
}

impl ModuleLoadError {
    pub fn new(module_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            referrer: None,
            message: message.into(),
        }
    }

    pub fn with_referrer(
        module_id: impl Into<String>,
        referrer: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            module_id: module_id.into(),
            referrer: Some(referrer.into()),
            message: message.into(),
        }
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn referrer(&self) -> Option<&str> {
        self.referrer.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn into_js_error(self, ctx: &JSContext) -> JSError {
        module_loader_error(ctx, self.to_string())
    }
}

impl fmt::Display for ModuleLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.referrer.as_deref() {
            Some(referrer) => write!(
                formatter,
                "failed to load module `{}` from `{}`: {}",
                self.module_id, referrer, self.message
            ),
            None => write!(
                formatter,
                "failed to load module `{}`: {}",
                self.module_id, self.message
            ),
        }
    }
}

impl std::error::Error for ModuleLoadError {}

/// Convert a typed module resolver return value into a JavaScriptCore key.
///
/// This trait is used by the ergonomic `#[module_resolver]` macro. Returning
/// `None` asks JavaScriptCore to treat the resolution as unhandled or failed.
pub trait IntoModuleResolveResult {
    /// Convert `self` into an optional resolved module key.
    ///
    /// # Errors
    /// Returns a JavaScript error when the conversion cannot create a valid
    /// JavaScriptCore string.
    fn into_module_resolve_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>>;
}

impl IntoModuleResolveResult for JSStringProtected {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(Some(self))
    }
}

impl IntoModuleResolveResult for JSString {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(Some(JSStringProtected::from(self.to_string())))
    }
}

impl IntoModuleResolveResult for String {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(Some(JSStringProtected::from(self)))
    }
}

impl IntoModuleResolveResult for &str {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(Some(JSStringProtected::from(self)))
    }
}

impl IntoModuleResolveResult for Option<JSStringProtected> {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(self)
    }
}

impl IntoModuleResolveResult for Option<JSString> {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(self.map(|value| JSStringProtected::from(value.to_string())))
    }
}

impl IntoModuleResolveResult for Option<String> {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(self.map(JSStringProtected::from))
    }
}

impl IntoModuleResolveResult for Option<&str> {
    fn into_module_resolve_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        Ok(self.map(JSStringProtected::from))
    }
}

impl<T> IntoModuleResolveResult for JSResult<T>
where
    T: IntoModuleResolveResult,
{
    fn into_module_resolve_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        self.and_then(|value| value.into_module_resolve_result(ctx))
    }
}

impl<T> IntoModuleResolveResult for Result<T, ModuleLoadError>
where
    T: IntoModuleResolveResult,
{
    fn into_module_resolve_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSStringProtected>> {
        self.map_err(|error| error.into_js_error(ctx))
            .and_then(|value| value.into_module_resolve_result(ctx))
    }
}

/// Convert a typed module fetcher return value into an owned module source.
///
/// This trait is used by `#[module_fetcher]`, which targets the preferred
/// `fetch_source` slot and can return JavaScript, JSON, or WebAssembly source.
pub trait IntoModuleSourceResult {
    /// Convert `self` into an optional owned module source.
    ///
    /// # Errors
    /// Returns a JavaScript error when source creation fails.
    fn into_module_source_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>>;
}

impl IntoModuleSourceResult for JSModuleSource {
    fn into_module_source_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        Ok(Some(self))
    }
}

impl IntoModuleSourceResult for ModuleSource {
    fn into_module_source_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        self.into_js_module_source().map(Some).ok_or_else(|| {
            module_loader_error(ctx, "failed to create JavaScriptCore module source")
        })
    }
}

impl IntoModuleSourceResult for Option<JSModuleSource> {
    fn into_module_source_result(
        self,
        _ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        Ok(self)
    }
}

impl IntoModuleSourceResult for Option<ModuleSource> {
    fn into_module_source_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        self.map(|source| source.into_module_source_result(ctx))
            .transpose()
            .map(Option::flatten)
    }
}

impl<T> IntoModuleSourceResult for JSResult<T>
where
    T: IntoModuleSourceResult,
{
    fn into_module_source_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        self.and_then(|value| value.into_module_source_result(ctx))
    }
}

impl<T> IntoModuleSourceResult for Result<T, ModuleLoadError>
where
    T: IntoModuleSourceResult,
{
    fn into_module_source_result(
        self,
        ctx: &JSContext,
    ) -> JSResult<Option<JSModuleSource>> {
        self.map_err(|error| error.into_js_error(ctx))
            .and_then(|value| value.into_module_source_result(ctx))
    }
}

/// Convert a typed import-meta provider return value into an object.
///
/// Returning `None` lets the wrapper return a null object reference to
/// JavaScriptCore. Most runtimes should return a plain object.
pub trait IntoImportMetaResult {
    /// Convert `self` into optional import-meta properties.
    ///
    /// # Errors
    /// Returns a JavaScript error when object creation or conversion fails.
    fn into_import_meta_result(self, ctx: &JSContext) -> JSResult<Option<JSObject>>;
}

impl IntoImportMetaResult for JSObject {
    fn into_import_meta_result(self, _ctx: &JSContext) -> JSResult<Option<JSObject>> {
        Ok(Some(self))
    }
}

impl IntoImportMetaResult for Option<JSObject> {
    fn into_import_meta_result(self, _ctx: &JSContext) -> JSResult<Option<JSObject>> {
        Ok(self)
    }
}

impl<T> IntoImportMetaResult for JSResult<T>
where
    T: IntoImportMetaResult,
{
    fn into_import_meta_result(self, ctx: &JSContext) -> JSResult<Option<JSObject>> {
        self.and_then(|value| value.into_import_meta_result(ctx))
    }
}

impl<T> IntoImportMetaResult for Result<T, ModuleLoadError>
where
    T: IntoImportMetaResult,
{
    fn into_import_meta_result(self, ctx: &JSContext) -> JSResult<Option<JSObject>> {
        self.map_err(|error| error.into_js_error(ctx))
            .and_then(|value| value.into_import_meta_result(ctx))
    }
}

fn module_loader_error(ctx: &JSContext, message: impl Into<JSString>) -> JSError {
    match JSError::new_typ(ctx, message) {
        Ok(error) | Err(error) => error,
    }
}

/// Owned low-level module source object for `JSModuleLoaderFetchSource`.
///
/// Returning [`JSModuleSource::into_raw`] from a fetch-source callback transfers
/// ownership to JavaScriptCore. If the value is not transferred, `Drop` releases
/// the underlying C API source object.
#[derive(Debug)]
pub struct JSModuleSource {
    inner: JSModuleSourceRef,
    _not_send_or_sync: NotSendOrSync,
}

impl JSModuleSource {
    pub fn javascript(source: &str) -> Option<Self> {
        let source = JSString::try_from(source.as_bytes()).ok()?;
        // SAFETY: `source.inner` is a live JSStringRef for the duration of the
        // call, and the C API copies the string into an owned module source.
        let inner = unsafe { JSModuleSourceCreateJavaScript(source.inner) };
        Self::from_ref(inner)
    }

    pub fn json(source: &str) -> Option<Self> {
        let source = JSString::try_from(source.as_bytes()).ok()?;
        // SAFETY: `source.inner` is a live JSStringRef for the duration of the
        // call, and the C API copies the string into an owned module source.
        let inner = unsafe { JSModuleSourceCreateJSON(source.inner) };
        Self::from_ref(inner)
    }

    pub fn webassembly(bytes: &[u8]) -> Option<Self> {
        let bytes_ptr = if bytes.is_empty() {
            std::ptr::null()
        } else {
            bytes.as_ptr()
        };
        // SAFETY: `bytes_ptr` either points to `bytes.len()` readable bytes or
        // is null for an empty slice, which the C API explicitly accepts. The C
        // API copies the bytes before returning.
        let inner = unsafe { JSModuleSourceCreateWebAssembly(bytes_ptr, bytes.len()) };
        Self::from_ref(inner)
    }

    fn from_ref(inner: JSModuleSourceRef) -> Option<Self> {
        if inner.is_null() {
            None
        } else {
            Some(Self {
                inner,
                _not_send_or_sync: not_send_or_sync(),
            })
        }
    }

    pub fn into_raw(self) -> JSModuleSourceRef {
        let inner = self.inner;
        std::mem::forget(self);
        inner
    }
}

impl Drop for JSModuleSource {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            // SAFETY: `inner` is owned by this RAII wrapper unless
            // `into_raw` consumed it with `mem::forget`; release is idempotent
            // only for null, which is checked above.
            unsafe { JSModuleSourceRelease(self.inner) };
        }
    }
}

/// Safe wrapper around JavaScriptCore module-loader callbacks.
///
/// `JSAPIModuleLoader` remains available for low-level users, but this type is
/// the recommended construction surface for host runtimes. It keeps the
/// optional callback slots explicit and works with [`JSContext::set_module_loader`].
#[derive(Debug, Clone, Copy)]
pub struct ModuleLoader {
    callbacks: JSAPIModuleLoader,
}

impl ModuleLoader {
    pub const fn builder() -> ModuleLoaderBuilder {
        ModuleLoaderBuilder::new()
    }

    pub const fn from_callbacks(callbacks: JSAPIModuleLoader) -> Self {
        Self { callbacks }
    }

    pub const fn callbacks(self) -> JSAPIModuleLoader {
        self.callbacks
    }

    /// Returns the default Rust-side filesystem module loader.
    pub const fn file_system() -> Self {
        Self::from_callbacks(file_module_loader())
    }
}

impl From<ModuleLoader> for JSAPIModuleLoader {
    fn from(loader: ModuleLoader) -> Self {
        loader.callbacks()
    }
}

impl From<JSAPIModuleLoader> for ModuleLoader {
    fn from(callbacks: JSAPIModuleLoader) -> Self {
        Self::from_callbacks(callbacks)
    }
}

/// Builder for module-loader callback sets.
#[derive(Debug, Clone, Copy)]
pub struct ModuleLoaderBuilder {
    callbacks: JSAPIModuleLoader,
}

impl ModuleLoaderBuilder {
    pub const fn new() -> Self {
        Self {
            callbacks: empty_module_loader(),
        }
    }

    pub const fn resolve(mut self, callback: JSModuleLoaderResolve) -> Self {
        self.callbacks.moduleLoaderResolve = callback;
        self
    }

    pub const fn evaluate(mut self, callback: JSModuleLoaderEvaluate) -> Self {
        self.callbacks.moduleLoaderEvaluate = callback;
        self
    }

    pub const fn fetch(mut self, callback: JSModuleLoaderFetch) -> Self {
        self.callbacks.moduleLoaderFetch = callback;
        self
    }

    pub const fn fetch_source(mut self, callback: JSModuleLoaderFetchSource) -> Self {
        self.callbacks.moduleLoaderFetchSource = callback;
        self
    }

    pub const fn import_meta(
        mut self,
        callback: JSModuleLoaderCreateImportMetaProperties,
    ) -> Self {
        self.callbacks.moduleLoaderCreateImportMetaProperties = callback;
        self
    }

    pub const fn build(self) -> ModuleLoader {
        ModuleLoader::from_callbacks(self.callbacks)
    }
}

impl Default for ModuleLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ModuleLoaderBuilder> for ModuleLoader {
    fn from(builder: ModuleLoaderBuilder) -> Self {
        builder.build()
    }
}

impl From<ModuleLoaderBuilder> for JSAPIModuleLoader {
    fn from(builder: ModuleLoaderBuilder) -> Self {
        builder.build().callbacks()
    }
}

const fn empty_module_loader() -> JSAPIModuleLoader {
    JSAPIModuleLoader {
        moduleLoaderResolve: None,
        moduleLoaderEvaluate: None,
        moduleLoaderFetch: None,
        moduleLoaderFetchSource: None,
        moduleLoaderCreateImportMetaProperties: None,
    }
}

fn is_dotted_relative_path(specifier: &str) -> bool {
    specifier.starts_with("./")
        || specifier.starts_with("../")
        || specifier.starts_with(".\\")
        || specifier.starts_with("..\\")
}

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("file://")?;
    Some(PathBuf::from(path))
}

fn path_to_file_url(path: &Path) -> Option<String> {
    let mut path = path.to_str()?.replace('\\', "/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    Some(format!("file://{path}"))
}

fn referrer_base_dir(referrer: &str) -> Option<PathBuf> {
    if referrer.is_empty() {
        return None;
    }

    let path = if referrer.starts_with("file://") {
        file_url_to_path(referrer)?
    } else {
        PathBuf::from(referrer)
    };

    path.parent().map(Path::to_path_buf)
}

fn module_specifier_to_path(specifier: &str, referrer: Option<&str>) -> Option<PathBuf> {
    if specifier.starts_with("file://") {
        return file_url_to_path(specifier);
    }

    let path = Path::new(specifier);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }

    if is_dotted_relative_path(specifier) {
        let base = referrer
            .and_then(referrer_base_dir)
            .or_else(|| std::env::current_dir().ok())?;
        return Some(base.join(path));
    }

    None
}

pub fn resolve_file_module_specifier(
    specifier: &str,
    referrer: Option<&str>,
) -> Option<String> {
    let path = module_specifier_to_path(specifier, referrer)?;
    let path = canonicalize_existing_path_or_parent(&path)?;
    path_to_file_url(&path)
}

fn canonicalize_existing_path_or_parent(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = std::fs::canonicalize(path) {
        return Some(path);
    }

    let parent = path.parent()?;
    let file_name = path.file_name()?;
    let parent = std::fs::canonicalize(parent).ok()?;
    Some(parent.join(file_name))
}

/// Reads source for a file module using the default file-loader policy.
///
/// Bare `.json` files are exposed as JavaScript modules with a default export.
/// Use [`read_file_module_source_for_import_type`] when handling an explicit
/// import type from a module fetch callback.
pub fn read_file_module_source(path: &Path) -> Option<String> {
    read_file_module_source_for_import_type(path, ModuleImportType::Unknown)
}

/// Reads source for a file module with an explicit import type.
///
/// For JSON imports requested through import attributes, the raw JSON is
/// returned so JavaScriptCore can parse it as a JSON module. Without that
/// explicit request, `.json` files are wrapped as JavaScript default exports for
/// compatibility with existing rust-jsc file-loader users.
pub fn read_file_module_source_for_import_type(
    path: &Path,
    import_type: ModuleImportType,
) -> Option<String> {
    match read_file_module(path, import_type)? {
        ModuleSource::JavaScript(source) | ModuleSource::Json(source) => Some(source),
        ModuleSource::WebAssembly(_) => None,
    }
}

pub fn read_file_module(
    path: &Path,
    import_type: ModuleImportType,
) -> Option<ModuleSource> {
    let bytes = std::fs::read(path).ok()?;
    let extension = path.extension().and_then(|extension| extension.to_str());
    if import_type == ModuleImportType::WebAssembly
        || extension == Some("wasm")
        || bytes.starts_with(b"\0asm")
    {
        return Some(ModuleSource::WebAssembly(bytes));
    }

    let source = String::from_utf8(bytes).ok()?;
    if import_type == ModuleImportType::Json {
        return Some(ModuleSource::Json(source));
    }

    if extension == Some("json") {
        return Some(ModuleSource::JavaScript(format!(
            "export default {source};"
        )));
    }

    Some(ModuleSource::JavaScript(source))
}

fn js_string_ref_from_utf8(value: &str) -> JSStringRef {
    let Ok(value) = CString::new(value.as_bytes()) else {
        return std::ptr::null_mut();
    };

    // SAFETY: `CString` guarantees a valid null-terminated byte buffer for the
    // duration of this call, and JavaScriptCore copies the string contents.
    unsafe { JSStringCreateWithUTF8CString(value.as_ptr()) }
}

unsafe extern "C" fn file_module_resolve(
    ctx: JSContextRef,
    key: JSValueRef,
    referrer: JSValueRef,
    _script_fetcher: JSValueRef,
) -> JSStringRef {
    if ctx.is_null() || key.is_null() {
        return std::ptr::null_mut();
    }

    let key = JSValue::new(key, ctx);
    let referrer = if referrer.is_null() {
        None
    } else {
        Some(JSValue::new(referrer, ctx))
    };

    let Some(key) = key.as_string().ok().map(|value| value.to_string()) else {
        return std::ptr::null_mut();
    };
    let referrer = referrer.and_then(|referrer| {
        if referrer.is_undefined() || referrer.is_null() {
            None
        } else {
            referrer.as_string().ok().map(|value| value.to_string())
        }
    });

    match resolve_file_module_specifier(&key, referrer.as_deref()) {
        Some(resolved) => js_string_ref_from_utf8(&resolved),
        None => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn file_module_fetch(
    ctx: JSContextRef,
    key: JSValueRef,
    attributes_value: JSValueRef,
    _script_fetcher: JSValueRef,
) -> JSStringRef {
    if ctx.is_null() || key.is_null() {
        return std::ptr::null_mut();
    }

    let key = JSValue::new(key, ctx);
    let Some(key) = key.as_string().ok().map(|value| value.to_string()) else {
        return std::ptr::null_mut();
    };
    let Some(path) = module_specifier_to_path(&key, None) else {
        return std::ptr::null_mut();
    };

    let import_type = ModuleImportType::from_attributes_value(ctx, attributes_value);
    match read_file_module_source_for_import_type(&path, import_type) {
        Some(source) => js_string_ref_from_utf8(&source),
        None => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn file_module_fetch_source(
    ctx: JSContextRef,
    key: JSValueRef,
    attributes_value: JSValueRef,
    _script_fetcher: JSValueRef,
) -> JSModuleSourceRef {
    if ctx.is_null() || key.is_null() {
        return std::ptr::null_mut();
    }

    let key = JSValue::new(key, ctx);
    let Some(key) = key.as_string().ok().map(|value| value.to_string()) else {
        return std::ptr::null_mut();
    };
    let Some(path) = module_specifier_to_path(&key, None) else {
        return std::ptr::null_mut();
    };

    let import_type = ModuleImportType::from_attributes_value(ctx, attributes_value);
    read_file_module(&path, import_type)
        .and_then(ModuleSource::into_js_module_source)
        .map(JSModuleSource::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

unsafe extern "C" fn file_module_import_meta(
    ctx: JSContextRef,
    key: JSValueRef,
    _script_fetcher: JSValueRef,
) -> JSObjectRef {
    if ctx.is_null() || key.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: JavaScriptCore invokes this callback with a live context for the
    // duration of import.meta creation, and this borrowed wrapper does not
    // retain or release the context.
    let ctx = unsafe { JSContext::borrowed(ctx) };
    let key = JSValue::new(key, ctx.inner);
    let object = JSObject::new(&ctx);
    let _ = object.set_property("url", &key, Default::default());
    object.into()
}

/// Returns rust-jsc's default filesystem module loader callbacks.
///
/// The loader supports absolute paths, `file://` URLs, `./`/`../` specifiers
/// relative to the importing module, bare JSON default exports, JSON import
/// attributes, and `.wasm` byte modules. This is deliberately a Rust-side policy
/// layer: install it with [`JSContext::set_module_loader`] when an embedding
/// wants this default, or provide custom callbacks for a runtime-specific
/// resolver and fetcher.
pub const fn file_module_loader() -> JSAPIModuleLoader {
    JSAPIModuleLoader {
        moduleLoaderResolve: Some(file_module_resolve),
        moduleLoaderEvaluate: None,
        moduleLoaderFetch: Some(file_module_fetch),
        moduleLoaderFetchSource: Some(file_module_fetch_source),
        moduleLoaderCreateImportMetaProperties: Some(file_module_import_meta),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn json_files_are_wrapped_as_default_exports() {
        let path = std::env::temp_dir()
            .join(format!("rust-jsc-json-module-{}.json", std::process::id()));
        fs::write(&path, r#"{"name":"rust-jsc"}"#).unwrap();

        let source = read_file_module_source(&path).unwrap();
        assert_eq!(source, r#"export default {"name":"rust-jsc"};"#);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn wasm_files_are_binary_module_sources() {
        let path = std::env::temp_dir()
            .join(format!("rust-jsc-wasm-module-{}.wasm", std::process::id()));
        fs::write(&path, b"\0asm\x01\0\0\0").unwrap();

        let source = read_file_module(&path, ModuleImportType::Unknown).unwrap();
        assert_eq!(
            source,
            ModuleSource::WebAssembly(b"\0asm\x01\0\0\0".to_vec())
        );
        assert!(read_file_module_source(&path).is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn bare_specifiers_are_not_file_policy() {
        assert!(module_specifier_to_path("@runtime/std", None).is_none());
    }

    #[test]
    fn source_url_can_resolve_against_existing_parent_without_existing_file() {
        let dir = std::env::temp_dir()
            .join(format!("rust-jsc-source-url-parent-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let resolved = resolve_file_module_specifier(
            dir.join("virtual-entry.js").to_str().unwrap(),
            None,
        )
        .unwrap();
        assert!(resolved.starts_with("file://"));
        assert!(resolved.ends_with("/virtual-entry.js"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn module_load_error_formats_module_and_referrer() {
        let error = ModuleLoadError::with_referrer(
            "@runtime/std",
            "file:///app/main.js",
            "module is disabled",
        );

        assert_eq!(error.module_id(), "@runtime/std");
        assert_eq!(error.referrer(), Some("file:///app/main.js"));
        assert_eq!(error.message(), "module is disabled");
        assert_eq!(
            error.to_string(),
            "failed to load module `@runtime/std` from `file:///app/main.js`: module is disabled"
        );
    }

    #[test]
    fn module_load_error_converts_through_loader_traits() {
        let ctx = JSContext::new();
        let result: Result<Option<String>, ModuleLoadError> =
            Err(ModuleLoadError::new("@runtime/std", "not found"));

        let error = match result.into_module_resolve_result(&ctx) {
            Ok(_) => panic!("expected module load error"),
            Err(error) => error,
        };
        assert_eq!(
            error.message().unwrap().to_string(),
            "failed to load module `@runtime/std`: not found"
        );
    }
}
