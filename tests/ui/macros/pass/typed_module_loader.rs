use rust_jsc::{
    module_fetcher, module_import_meta_provider, module_resolver, JSContext,
    JSObject, JSResult, JSValue, ModuleImportType, ModuleSource,
};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    referrer: Option<String>,
) -> JSResult<Option<String>> {
    Ok(Some(referrer.unwrap_or(specifier)))
}

#[module_fetcher]
fn fetch(
    _ctx: JSContext,
    key: String,
    import_type: ModuleImportType,
) -> JSResult<Option<ModuleSource>> {
    let source = match import_type {
        ModuleImportType::Json => ModuleSource::Json(format!("{{\"key\":{key:?}}}")),
        ModuleImportType::WebAssembly => return Ok(None),
        ModuleImportType::Unknown | ModuleImportType::JavaScript => {
            ModuleSource::JavaScript(format!("export default {key:?};"))
        }
    };
    Ok(Some(source))
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let object = JSObject::new(&ctx);
    object.set_property(
        "url",
        &JSValue::string(&ctx, key),
        Default::default(),
    )?;
    Ok(Some(object))
}

fn main() {
    let _ = resolve;
    let _ = fetch;
    let _ = import_meta;
}
