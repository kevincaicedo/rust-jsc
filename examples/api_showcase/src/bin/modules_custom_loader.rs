use std::{thread, time::Duration};

use rust_jsc::{
    module_fetcher, module_import_meta_provider, module_resolver, JSContext, JSObject,
    JSResult, JSValue, ModuleImportType, ModuleLoadError, ModuleLoaderBuilder,
    ModuleSource,
};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> Result<Option<String>, ModuleLoadError> {
    Ok(Some(specifier))
}

#[module_fetcher]
fn fetch(
    _ctx: JSContext,
    key: String,
    import_type: ModuleImportType,
) -> Result<Option<ModuleSource>, ModuleLoadError> {
    match (key.as_str(), import_type) {
        ("virtual:settings", ModuleImportType::Json) => Ok(Some(ModuleSource::Json(
            r#"{"name":"custom-json"}"#.to_string(),
        ))),
        ("virtual:math", ModuleImportType::Unknown | ModuleImportType::JavaScript) => {
            Ok(Some(ModuleSource::JavaScript(
                "export const add = (left, right) => left + right;".to_string(),
            )))
        }
        _ => Err(ModuleLoadError::new(key, "module is not registered")),
    }
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let meta = JSObject::new(&ctx);
    meta.set_property(
        "url",
        &JSValue::string(&ctx, format!("custom://{key}")),
        Default::default(),
    )?;
    Ok(Some(meta))
}

fn wait_for_string(ctx: &JSContext, name: &str) -> Option<String> {
    for _ in 0..50 {
        ctx.run_deferred_work();
        ctx.run_microtasks();
        let value = ctx
            .evaluate_script(&format!("globalThis.{name}"), None)
            .unwrap();
        if value.is_string() {
            return Some(value.as_string().unwrap().to_string());
        }
        thread::sleep(Duration::from_millis(10));
    }
    None
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    ctx.set_module_loader(
        ModuleLoaderBuilder::new()
            .resolve(Some(resolve))
            .fetch_source(Some(fetch))
            .import_meta(Some(import_meta)),
    );

    let promise = ctx.evaluate_module_from_source(
        r#"
        import { add } from "virtual:math";
        import settings from "virtual:settings" with { type: "json" };

        globalThis.customLoaderSummary =
            `${settings.name}:${add(20, 22)}:${import.meta.url}`;
        "#,
        "virtual:entry",
        None,
    )?;
    ctx.global_object().set_property(
        "__customModulePromise",
        &promise,
        Default::default(),
    )?;
    ctx.evaluate_script(
        r#"
        __customModulePromise.catch((error) => {
            globalThis.customLoaderError = String(error && (error.stack || error));
        });
        "#,
        None,
    )?;

    let summary = wait_for_string(&ctx, "customLoaderSummary")
        .expect("custom loader did not set summary");
    assert_eq!(summary, "custom-json:42:custom://virtual:entry");

    let error = ctx.evaluate_script("globalThis.customLoaderError", None)?;
    assert!(error.is_undefined(), "custom loader failed: {error:?}");

    println!("modules custom loader: {summary}");
    Ok(())
}
