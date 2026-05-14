use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use rust_jsc::{
    module_resolve, JSArray, JSContext, JSModuleLoader, JSObject, JSStringProtected,
    JSTypedArray, JSTypedArrayType, JSValue, ModuleLoader,
};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("modules")
}

fn drain(ctx: &JSContext) {
    for _ in 0..8 {
        ctx.run_deferred_work();
        ctx.run_microtasks();
    }
}

fn wait_for_string(ctx: &JSContext, name: &str) -> Option<String> {
    for _ in 0..50 {
        drain(ctx);
        let value = ctx
            .evaluate_script(&format!("globalThis.{name}"), None)
            .unwrap();
        if value.is_string() {
            return Some(value.as_string().unwrap().to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

#[test]
fn file_module_fixture_loads_real_js_json_dynamic_and_import_meta() {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let entry = fixture_dir().join("nested").join("real_file_entry.js");
    let promise = ctx
        .evaluate_module(entry.to_string_lossy().as_ref())
        .unwrap();
    assert!(promise.is_object());
    drain(&ctx);

    let summary = ctx
        .evaluate_script("globalThis.realFileSummary", None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string();
    assert_eq!(summary, "fixture-math:fixture-json:12");

    let meta = ctx
        .evaluate_script("globalThis.realFileMeta", None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string();
    assert!(
        meta.starts_with("file://"),
        "unexpected import.meta.url: {meta}"
    );
    assert!(meta.ends_with("/nested/real_file_entry.js"));

    let dynamic = wait_for_string(&ctx, "realFileDynamic")
        .expect("dynamic file import did not settle");
    assert_eq!(dynamic, "dynamic-fixture:true");
    let dynamic_error = ctx
        .evaluate_script("globalThis.realFileDynamicError", None)
        .unwrap();
    assert!(
        dynamic_error.is_undefined(),
        "dynamic import failed: {dynamic_error:?}"
    );
}

#[test]
fn source_module_uses_source_url_to_resolve_real_file_dependencies() {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let source_url = fixture_dir().join("source_entry.js");
    let promise = ctx
        .evaluate_module_from_source(
            r#"
                import { add, label } from "./math.js";
                import config from "./config.json" with { type: "json" };
                globalThis.sourceFixtureSummary = `${label}:${config.name}:${add(30, 12)}`;
                export const value = globalThis.sourceFixtureSummary;
            "#,
            source_url.to_string_lossy().as_ref(),
            None,
        )
        .unwrap();
    assert!(promise.is_object());
    ctx.global_object()
        .set_property("sourceModulePromise", &promise, Default::default())
        .unwrap();
    ctx.evaluate_script(
        r#"
        globalThis.sourceModuleError = undefined;
        globalThis.sourceModulePromise.then(
            () => { globalThis.sourceModuleDone = true; },
            error => { globalThis.sourceModuleError = String(error && (error.stack || error.message) || error); },
        );
        "#,
        None,
    )
    .unwrap();
    drain(&ctx);

    let summary = ctx
        .evaluate_script("globalThis.sourceFixtureSummary", None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string();
    let error = ctx
        .evaluate_script("globalThis.sourceModuleError", None)
        .unwrap();
    let error = if error.is_string() {
        error.as_string().unwrap().to_string()
    } else {
        format!("{error:?}")
    };
    assert_eq!(
        summary, "fixture-math:fixture-json:42",
        "source module promise rejected with {error}"
    );
}

#[test]
fn load_then_link_and_evaluate_real_file_module() {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let entry = fixture_dir().join("nested").join("real_file_entry.js");
    let key = entry.to_string_lossy();
    let load_promise = ctx.load_module(key.as_ref()).unwrap();
    assert!(load_promise.is_object());
    drain(&ctx);

    let evaluate_promise = ctx.link_and_evaluate_module(key.as_ref()).unwrap();
    assert!(evaluate_promise.is_object());
    drain(&ctx);

    let summary = ctx
        .evaluate_script("globalThis.realFileSummary", None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string();
    assert_eq!(summary, "fixture-math:fixture-json:12");
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
fn synthetic_module_rejects_duplicates_then_imports_default_and_named_exports() {
    let ctx = JSContext::new();
    let default_value = JSValue::string(&ctx, "default-fixture");
    let named_value = JSValue::number(&ctx, 99.0);

    let duplicate = ctx.create_synthetic_module(
        "@runtime/duplicate",
        &[("value", &named_value), ("value", &default_value)],
    );
    assert!(duplicate.is_err());

    ctx.create_synthetic_module(
        "@runtime/config",
        &[("default", &default_value), ("answer", &named_value)],
    )
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
            import config, { answer } from "@runtime/config";
            globalThis.syntheticFixtureSummary = `${config}:${answer}`;
        "#,
        "synthetic-fixture.js",
        None,
    )
    .unwrap();
    drain(&ctx);

    let summary = ctx
        .evaluate_script("globalThis.syntheticFixtureSummary", None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string();
    assert_eq!(summary, "default-fixture:99");
}

#[test]
fn array_method_and_typed_array_api_stress_paths() {
    let ctx = JSContext::new();
    let array = JSArray::new_array(&ctx, &[]).unwrap();
    for i in 0..128 {
        let value = JSValue::number(&ctx, i as f64);
        assert_eq!(array.push(&value).unwrap(), i + 1);
        assert_eq!(array.length().unwrap(), i + 1);
    }

    let object = ctx
        .evaluate_script(
            "({ base: 40, add(value) { return this.base + value; } })",
            None,
        )
        .unwrap()
        .as_object()
        .unwrap();
    let result = object
        .call_method("add", &[JSValue::number(&ctx, 2.0)])
        .unwrap();
    assert_eq!(result.as_number().unwrap(), 42.0);

    let missing = object.call_method("missing", &[]);
    assert!(missing.is_err());

    let mut bytes = (0u16..64).collect::<Vec<_>>();
    let typed_array = JSTypedArray::with_bytes(
        &ctx,
        bytes.as_mut_slice(),
        JSTypedArrayType::Uint16Array,
    )
    .unwrap();
    assert_eq!(typed_array.len().unwrap(), 64);
    assert!(!typed_array.is_empty().unwrap());
    assert_eq!(typed_array.byte_len().unwrap(), 128);
    assert_eq!(typed_array.as_vec::<u16>().unwrap()[42], 42);

    let empty = JSTypedArray::new(&ctx, 0).unwrap();
    assert!(empty.is_empty().unwrap());

    let plain_object = JSObject::new(&ctx);
    assert!(JSTypedArray::from_value(&plain_object.into()).is_err());
}
