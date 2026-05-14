use std::{path::PathBuf, time::Duration};

use rust_jsc::{
    module_resolver, JSContext, JSValue, ModuleLoadError, ModuleLoader,
    ModuleLoaderBuilder,
};

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

fn read_global_string(ctx: &JSContext, name: &str) -> String {
    ctx.evaluate_script(&format!("globalThis.{name}"), None)
        .unwrap()
        .as_string()
        .unwrap()
        .to_string()
}

fn module_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules")
}

#[module_resolver]
fn synthetic_resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> Result<Option<String>, ModuleLoadError> {
    Ok(Some(specifier))
}

fn run_file_loader_example() {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let entry = module_dir().join("main.js");
    let promise = ctx
        .evaluate_module(entry.to_string_lossy().as_ref())
        .unwrap();
    assert!(promise.is_object());
    drain(&ctx);

    println!(
        "file static: {}",
        read_global_string(&ctx, "exampleStaticSummary")
    );
    println!(
        "import.meta: {}",
        read_global_string(&ctx, "exampleImportMeta")
    );
    println!(
        "file dynamic: {}",
        wait_for_string(&ctx, "exampleDynamicSummary")
            .expect("dynamic import did not settle")
    );
}

fn run_source_module_example() {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let source_url = module_dir().join("source-entry.js");
    let promise = ctx
        .evaluate_module_from_source(
            r#"
                import { add, label } from "./math.js";
                import settings from "./settings.json" with { type: "json" };
                globalThis.sourceSummary = `${label}:${settings.name}:${add(1, 2)}`;
            "#,
            source_url.to_string_lossy().as_ref(),
            None,
        )
        .unwrap();
    assert!(promise.is_object());
    drain(&ctx);

    println!(
        "source module: {}",
        read_global_string(&ctx, "sourceSummary")
    );
}

fn run_synthetic_module_example() {
    let ctx = JSContext::new();
    let name = JSValue::string(&ctx, "synthetic-config");
    let answer = JSValue::number(&ctx, 42.0);
    ctx.create_synthetic_module(
        "@runtime/config",
        &[("default", &name), ("answer", &answer)],
    )
    .unwrap();
    ctx.set_module_loader(ModuleLoaderBuilder::new().resolve(Some(synthetic_resolve)));

    let promise = ctx
        .evaluate_module_from_source(
            r#"
                import config, { answer } from "@runtime/config";
                globalThis.syntheticSummary = `${config}:${answer}`;
            "#,
            "synthetic-entry.js",
            None,
        )
        .unwrap();
    assert!(promise.is_object());
    drain(&ctx);

    println!(
        "synthetic: {}",
        read_global_string(&ctx, "syntheticSummary")
    );
}

fn main() {
    run_file_loader_example();
    run_source_module_example();
    run_synthetic_module_example();
}
