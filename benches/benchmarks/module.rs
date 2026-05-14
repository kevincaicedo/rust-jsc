use criterion::{black_box, criterion_group, Criterion};
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
        ("bench:math", ModuleImportType::Unknown | ModuleImportType::JavaScript) => {
            Ok(Some(ModuleSource::JavaScript(
                "export const add = (left, right) => left + right;".to_string(),
            )))
        }
        ("bench:settings", ModuleImportType::Json) => Ok(Some(ModuleSource::Json(
            r#"{"name":"benchmark"}"#.to_string(),
        ))),
        _ => Err(ModuleLoadError::new(key, "module is not registered")),
    }
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let meta = JSObject::new(&ctx);
    meta.set_property(
        "url",
        &JSValue::string(&ctx, format!("bench://{key}")),
        Default::default(),
    )?;
    Ok(Some(meta))
}

fn pump_module(ctx: &JSContext) {
    ctx.run_deferred_work();
    ctx.run_microtasks();
}

fn bench_module_source_eval(c: &mut Criterion) {
    let ctx = JSContext::new();
    let mut index = 0u64;

    c.bench_function("module_source_eval", |b| {
        b.iter(|| {
            index += 1;
            let key = format!("bench:inline:{index}");
            let promise = ctx
                .evaluate_module_from_source("export const value = 42;", &key, None)
                .unwrap();
            pump_module(&ctx);
            black_box(promise);
        });
    });
}

fn bench_synthetic_module_create(c: &mut Criterion) {
    let ctx = JSContext::new();
    let mut index = 0u64;

    c.bench_function("synthetic_module_create", |b| {
        b.iter(|| {
            index += 1;
            let key = format!("bench:synthetic:{index}");
            let value = JSValue::number(&ctx, index as f64);
            let module = ctx
                .create_synthetic_module(&key, &[("default", &value), ("value", &value)])
                .unwrap();
            black_box(module);
        });
    });
}

fn bench_custom_module_loader_eval(c: &mut Criterion) {
    let ctx = JSContext::new();
    ctx.set_module_loader(
        ModuleLoaderBuilder::new()
            .resolve(Some(resolve))
            .fetch_source(Some(fetch))
            .import_meta(Some(import_meta)),
    );
    let mut index = 0u64;

    c.bench_function("custom_module_loader_eval", |b| {
        b.iter(|| {
            index += 1;
            let key = format!("bench:entry:{index}");
            let promise = ctx
                .evaluate_module_from_source(
                    r#"
                    import { add } from "bench:math";
                    import settings from "bench:settings" with { type: "json" };
                    globalThis.__benchModuleResult =
                        `${settings.name}:${add(20, 22)}:${import.meta.url}`;
                    "#,
                    &key,
                    None,
                )
                .unwrap();
            pump_module(&ctx);
            black_box(promise);
        });
    });
}

criterion_group!(
    benches,
    bench_module_source_eval,
    bench_synthetic_module_create,
    bench_custom_module_loader_eval,
);
