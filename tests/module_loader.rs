use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rust_jsc::{
    module_fetcher, module_import_meta_provider, module_loader, module_resolve,
    module_resolver, JSContext, JSModuleLoader, JSObject, JSResult, JSStringProtected,
    JSValue, ModuleImportType, ModuleLoader, ModuleSource,
};

fn temp_module_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("rust-jsc-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn write(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
}

fn write_bytes(path: &Path, source: &[u8]) {
    fs::write(path, source).unwrap();
}

fn file_url(path: &Path) -> String {
    let mut path = path.to_string_lossy().replace('\\', "/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    format!("file://{path}")
}

fn drain_microtasks(ctx: &JSContext) {
    for _ in 0..8 {
        ctx.run_microtasks();
    }
}

fn global_string(ctx: &JSContext, name: &str) -> String {
    ctx.evaluate_script(&format!("globalThis.{name}"), None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string()
}

fn wait_for_global_number(ctx: &JSContext, name: &str, expected: f64) -> bool {
    for _ in 0..100 {
        ctx.run_deferred_work();
        drain_microtasks(ctx);
        let value = ctx
            .evaluate_script(&format!("globalThis.{name}"), None)
            .unwrap();
        if value.is_number() && value.as_number().unwrap() == expected {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

const WASM_ANSWER_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
    0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, 0x61, 0x6e, 0x73, 0x77, 0x65,
    0x72, 0x00, 0x00, 0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
];

#[test]
fn file_loader_static_js_json_and_import_meta() {
    let dir = temp_module_dir("file-static");
    write(&dir.join("dep.js"), "export const label = 'js-dep';");
    write(&dir.join("data.json"), r#"{"name":"rust-jsc"}"#);
    write(
        &dir.join("main.js"),
        r#"
            import { label } from './dep.js';
            import data from './data.json';
            globalThis.staticSummary = `${label}:${data.name}:${import.meta.url.startsWith('file://')}`;
        "#,
    );

    let raw = module_loader::file_module_loader();
    let loader = ModuleLoader::builder()
        .resolve(raw.moduleLoaderResolve)
        .fetch(raw.moduleLoaderFetch)
        .fetch_source(raw.moduleLoaderFetchSource)
        .import_meta(raw.moduleLoaderCreateImportMetaProperties)
        .build();
    let ctx = JSContext::new();
    ctx.set_module_loader(loader);
    ctx.evaluate_module(dir.join("main.js").to_string_lossy().as_ref())
        .unwrap();
    drain_microtasks(&ctx);

    assert_eq!(global_string(&ctx, "staticSummary"), "js-dep:rust-jsc:true");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_loader_json_import_attributes_return_raw_json() {
    let dir = temp_module_dir("json-attributes");
    write(&dir.join("data.json"), r#"{"name":"rust-jsc","count":2}"#);
    write(
        &dir.join("main.js"),
        r#"
            import data from './data.json' with { type: 'json' };
            globalThis.attrJsonSummary = `${data.name}:${data.count}`;
        "#,
    );

    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());
    ctx.evaluate_module(dir.join("main.js").to_string_lossy().as_ref())
        .unwrap();
    drain_microtasks(&ctx);

    assert_eq!(global_string(&ctx, "attrJsonSummary"), "rust-jsc:2");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_loader_resolves_file_url_entry_and_nested_relative_imports() {
    let dir = temp_module_dir("file-url-nested");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).unwrap();
    write(&dir.join("data.json"), r#"{"name":"nested-json"}"#);
    write(
        &nested.join("dep.js"),
        r#"
            import data from '../data.json';
            export const nestedName = data.name;
        "#,
    );
    write(
        &dir.join("main.js"),
        r#"
            import { nestedName } from './nested/dep.js';
            globalThis.fileUrlNestedSummary = nestedName;
        "#,
    );

    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());
    ctx.evaluate_module(&file_url(&dir.join("main.js")))
        .unwrap();
    drain_microtasks(&ctx);

    assert_eq!(global_string(&ctx, "fileUrlNestedSummary"), "nested-json");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_loader_dynamic_imports_js_and_json_with_attributes() {
    let dir = temp_module_dir("dynamic");
    write(&dir.join("dep.js"), "export const value = 'dynamic-js';");
    write(&dir.join("data.json"), r#"{"name":"dynamic-json"}"#);
    write(
        &dir.join("main.js"),
        r#"
            Promise.all([
                import('./dep.js'),
                import('./data.json', { with: { type: 'json' } }),
            ]).then(
                ([dep, json]) => { globalThis.dynamicSummary = `${dep.value}:${json.default.name}`; },
                error => { globalThis.dynamicError = String(error && error.message || error); },
            );
        "#,
    );

    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());
    ctx.evaluate_module(dir.join("main.js").to_string_lossy().as_ref())
        .unwrap();
    drain_microtasks(&ctx);

    let error = ctx
        .evaluate_script("globalThis.dynamicError", None)
        .unwrap();
    assert!(error.is_undefined(), "dynamic import failed: {error:?}");
    assert_eq!(
        global_string(&ctx, "dynamicSummary"),
        "dynamic-js:dynamic-json"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_loader_static_and_dynamic_webassembly_modules() {
    let ctx = JSContext::new();
    assert!(
        ctx
        .evaluate_script("typeof WebAssembly !== 'undefined'", None)
        .unwrap()
        .as_boolean(),
        "JavaScriptCore was built without WebAssembly support; rust-jsc WebAssembly module support requires ENABLE_WEBASSEMBLY"
    );

    let dir = temp_module_dir("wasm");
    write_bytes(&dir.join("answer.wasm"), WASM_ANSWER_MODULE);
    write(
        &dir.join("main.js"),
        r#"
            import { answer } from './answer.wasm';
            globalThis.wasmStatic = answer();
            import('./answer.wasm').then(
                module => { globalThis.wasmDynamic = module.answer(); },
                error => { globalThis.wasmDynamicError = String(error && error.message || error); },
            );
        "#,
    );

    ctx.set_module_loader(ModuleLoader::file_system());
    let promise = ctx
        .evaluate_module(dir.join("main.js").to_string_lossy().as_ref())
        .unwrap();
    ctx.global_object()
        .set_property("__wasmModulePromise", &promise, Default::default())
        .unwrap();
    ctx.evaluate_script(
        r#"
            globalThis.__wasmModulePromise.then(
                () => { globalThis.wasmEvaluationDone = true; },
                error => { globalThis.wasmEvaluationError = String(error && error.message || error); },
            );
        "#,
        None,
    )
    .unwrap();
    drain_microtasks(&ctx);

    let static_ready = wait_for_global_number(&ctx, "wasmStatic", 42.0);
    let dynamic_ready = wait_for_global_number(&ctx, "wasmDynamic", 42.0);
    let evaluation_error = ctx
        .evaluate_script("globalThis.wasmEvaluationError", None)
        .unwrap();
    assert!(
        evaluation_error.is_undefined(),
        "wasm module evaluation failed: {evaluation_error:?}"
    );
    let error = ctx
        .evaluate_script("globalThis.wasmDynamicError", None)
        .unwrap();
    assert!(
        error.is_undefined(),
        "dynamic wasm import failed: {error:?}"
    );
    assert!(static_ready, "wasm static export was not initialized");
    assert!(dynamic_ready, "wasm dynamic export was not initialized");
    let _ = fs::remove_dir_all(dir);
}

#[module_resolver]
fn typed_resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> JSResult<Option<String>> {
    Ok(Some(specifier))
}

#[module_fetcher]
fn typed_fetch(
    _ctx: JSContext,
    key: String,
    import_type: ModuleImportType,
) -> JSResult<Option<ModuleSource>> {
    let source = match (key.as_str(), import_type) {
        ("typed:data", ModuleImportType::Json) => {
            ModuleSource::Json(r#"{"name":"typed-json"}"#.to_string())
        }
        ("typed:dep", ModuleImportType::Unknown | ModuleImportType::JavaScript) => {
            ModuleSource::JavaScript("export default 'typed-js';".to_string())
        }
        _ => return Ok(None),
    };
    Ok(Some(source))
}

#[module_import_meta_provider]
fn typed_import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let object = JSObject::new(&ctx);
    object.set_property("url", &JSValue::string(&ctx, key), Default::default())?;
    Ok(Some(object))
}

#[test]
fn typed_module_loader_macros_handle_source_json_and_import_meta() {
    let ctx = JSContext::new();
    ctx.set_module_loader(
        ModuleLoader::builder()
            .resolve(Some(typed_resolve))
            .fetch_source(Some(typed_fetch))
            .import_meta(Some(typed_import_meta))
            .build(),
    );

    ctx.evaluate_module_from_source(
        r#"
            import label from 'typed:dep';
            import data from 'typed:data' with { type: 'json' };
            globalThis.typedLoaderSummary = `${label}:${data.name}:${import.meta.url}`;
        "#,
        "typed-entry.js",
        None,
    )
    .unwrap();
    drain_microtasks(&ctx);

    assert_eq!(
        global_string(&ctx, "typedLoaderSummary"),
        "typed-js:typed-json:typed-entry.js"
    );
}

#[module_resolve]
fn synthetic_resolve(
    _ctx: JSContext,
    key: JSValue,
    _referrer: JSValue,
    _script_fetcher: JSValue,
) -> JSStringProtected {
    JSStringProtected::from(key.as_string().unwrap().to_string())
}

#[test]
fn synthetic_module_imports_work_for_static_and_dynamic_sources() {
    let ctx = JSContext::new();
    let name = JSValue::string(&ctx, "synthetic-rust");
    ctx.create_synthetic_module("@runtime/config", &[("name", &name)])
        .unwrap();

    ctx.set_module_loader(JSModuleLoader {
        moduleLoaderResolve: Some(synthetic_resolve),
        moduleLoaderEvaluate: None,
        moduleLoaderFetch: None,
        moduleLoaderFetchSource: None,
        moduleLoaderCreateImportMetaProperties: None,
    });

    ctx.evaluate_module_from_source(
        r#"
            import { name } from '@runtime/config';
            globalThis.syntheticStatic = name;
            import('@runtime/config').then(
                module => { globalThis.syntheticDynamic = module.name; },
                error => { globalThis.syntheticDynamicError = String(error && error.message || error); },
            );
        "#,
        "synthetic-entry.js",
        None,
    )
    .unwrap();
    drain_microtasks(&ctx);

    let error = ctx
        .evaluate_script("globalThis.syntheticDynamicError", None)
        .unwrap();
    assert!(
        error.is_undefined(),
        "synthetic dynamic import failed: {error:?}"
    );
    assert_eq!(global_string(&ctx, "syntheticStatic"), "synthetic-rust");
    assert_eq!(global_string(&ctx, "syntheticDynamic"), "synthetic-rust");
}
