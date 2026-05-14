use rust_jsc::{
    callback, module_import_meta_provider, module_resolver, JSContext, JSFunction,
    JSObject, JSResult, JSValue, ModuleLoadError, ModuleLoaderBuilder,
};

#[callback]
fn log_info(message: String) {
    println!("INFO: {message}");
}

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> Result<Option<String>, ModuleLoadError> {
    Ok(Some(specifier))
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let meta = JSObject::new(&ctx);
    meta.set_property(
        "url",
        &JSValue::string(&ctx, format!("hello://{key}")),
        Default::default(),
    )?;
    Ok(Some(meta))
}

fn install_console(ctx: &JSContext) -> JSResult<()> {
    let console = JSObject::new(ctx);
    let log = JSFunction::callback(ctx, Some("log"), Some(log_info));
    let log_value: JSValue = log.into();

    console.set_property("log", &log_value, Default::default())?;
    let console_value: JSValue = console.into();
    ctx.global_object()
        .set_property("console", &console_value, Default::default())
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    ctx.set_inspectable(true);
    install_console(&ctx)?;
    ctx.set_module_loader(
        ModuleLoaderBuilder::new()
            .resolve(Some(resolve))
            .import_meta(Some(import_meta)),
    );

    let name = JSValue::string(&ctx, "John Doe");
    let default = JSObject::new(&ctx);
    default.set_property("name", &name, Default::default())?;
    let default_value: JSValue = default.into();
    ctx.create_synthetic_module(
        "@rust-jsc",
        &[("default", &default_value), ("name", &name)],
    )?;

    let promise = ctx.evaluate_module_from_source(
        r#"
        import lib, { name } from "@rust-jsc";
        console.log(`Virtual: ${lib.name} - ${name}`);
        globalThis.exampleName = name;
        globalThis.exampleMeta = import.meta.url;
        "#,
        "hello_world.js",
        None,
    )?;
    ctx.global_object()
        .set_property("__module_promise", &promise, Default::default())?;
    ctx.run_deferred_work();
    ctx.run_microtasks();

    let example_name = ctx
        .evaluate_script("globalThis.exampleName", None)?
        .as_string()?
        .to_string();
    println!("Example module name: {example_name}");

    Ok(())
}
