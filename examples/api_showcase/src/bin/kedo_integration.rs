use std::collections::VecDeque;

use rust_jsc::{
    callback, module_fetcher, module_import_meta_provider, module_resolver,
    CallbackContext, JSContext, JSFunction, JSObject, JSResult, JSValue,
    ModuleImportType, ModuleLoadError, ModuleLoaderBuilder, ModuleSource, OwnedJSContext,
    Promise, PromiseResolver, UnhandledRejectionHandler,
};

enum HostTask {
    ResolvePromise(PromiseResolver, String),
}

struct HostRuntime {
    ctx: OwnedJSContext,
    tasks: VecDeque<HostTask>,
    _unhandled: UnhandledRejectionHandler,
}

#[callback]
fn runtime_log(ctx: CallbackContext, value: JSValue) -> JSResult<()> {
    ctx.global_object()
        .set_property("lastLog", &value, Default::default())
}

#[callback]
fn record_unhandled(
    ctx: CallbackContext,
    _promise: JSValue,
    reason: JSValue,
) -> JSResult<()> {
    ctx.global_object()
        .set_property("unhandledReason", &reason, Default::default())
}

#[module_resolver]
fn resolve_runtime_module(
    _ctx: JSContext,
    specifier: String,
    _referrer: Option<String>,
) -> Result<Option<String>, ModuleLoadError> {
    Ok(Some(specifier))
}

#[module_fetcher]
fn fetch_runtime_module(
    _ctx: JSContext,
    key: String,
    import_type: ModuleImportType,
) -> Result<Option<ModuleSource>, ModuleLoadError> {
    if key != "@std/env" {
        return Err(ModuleLoadError::new(key, "no source for runtime module"));
    }

    if matches!(
        import_type,
        ModuleImportType::Json | ModuleImportType::WebAssembly
    ) {
        return Err(ModuleLoadError::new(
            key,
            "unsupported import type for @std/env",
        ));
    }

    Ok(Some(ModuleSource::JavaScript(
        "export const runtime = 'kedojs'; export const channel = 'public-api';"
            .to_string(),
    )))
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let meta = JSObject::new(&ctx);
    meta.set_property(
        "url",
        &JSValue::string(&ctx, format!("kedo://{key}")),
        Default::default(),
    )?;
    Ok(Some(meta))
}

impl HostRuntime {
    fn new() -> JSResult<Self> {
        let ctx = JSContext::new();
        ctx.set_module_loader(
            ModuleLoaderBuilder::new()
                .resolve(Some(resolve_runtime_module))
                .fetch_source(Some(fetch_runtime_module))
                .import_meta(Some(import_meta)),
        );

        let unhandled_function =
            JSFunction::callback(&ctx, Some("recordUnhandled"), Some(record_unhandled));
        let unhandled_object: JSObject = unhandled_function.into();
        let unhandled = ctx.set_unhandled_rejection_handler(&unhandled_object)?;

        let mut runtime = Self {
            ctx,
            tasks: VecDeque::new(),
            _unhandled: unhandled,
        };
        runtime.install_console()?;
        Ok(runtime)
    }

    fn install_console(&mut self) -> JSResult<()> {
        let console = JSObject::new(&self.ctx);
        let log = JSFunction::callback(&self.ctx, Some("log"), Some(runtime_log));
        let log_value: JSValue = log.into();
        console.set_property("log", &log_value, Default::default())?;

        let console_value: JSValue = console.into();
        self.ctx.global_object().set_property(
            "console",
            &console_value,
            Default::default(),
        )
    }

    fn start(&mut self) -> JSResult<()> {
        let (promise, resolver) = Promise::new_pending(&self.ctx)?;
        let promise_value: JSValue = promise.into();
        self.ctx.global_object().set_property(
            "hostReady",
            &promise_value,
            Default::default(),
        )?;
        self.tasks
            .push_back(HostTask::ResolvePromise(resolver, "ready".to_string()));

        let module_promise = self.ctx.evaluate_module_from_source(
            r#"
            import { runtime, channel } from "@std/env";

            globalThis.kedoLoaded = `${runtime}:${channel}:${import.meta.url}`;
            hostReady.then((value) => {
                console.log(`${runtime}:${channel}:${import.meta.url}:${value}`);
                globalThis.kedoSummary = globalThis.lastLog;
            }).catch((error) => {
                globalThis.kedoError = String(error && (error.stack || error));
            });
            "#,
            "app.js",
            None,
        )?;
        self.ctx.global_object().set_property(
            "__app_promise",
            &module_promise,
            Default::default(),
        )?;
        self.ctx.evaluate_script(
            r#"
            __app_promise.catch((error) => {
                globalThis.moduleError = String(error && (error.stack || error));
            });
            "#,
            None,
        )?;
        Ok(())
    }

    fn tick(&mut self) -> JSResult<()> {
        self.ctx.run_deferred_work();
        self.ctx.run_microtasks();

        if let Some(task) = self.tasks.pop_front() {
            match task {
                HostTask::ResolvePromise(resolver, value) => {
                    resolver.resolve(None, &[JSValue::string(&self.ctx, value)])?;
                }
            }
        }

        self.ctx.run_deferred_work();
        self.ctx.run_microtasks();
        Ok(())
    }

    fn summary(&self) -> JSResult<String> {
        self.ctx
            .evaluate_script("globalThis.kedoSummary", None)?
            .as_string()
            .map(|value| value.to_string())
    }

    fn debug_state(&self) -> JSResult<String> {
        self.ctx
            .evaluate_script(
                "`${String(globalThis.kedoLoaded)}|${String(globalThis.kedoError)}|${String(globalThis.moduleError)}`",
                None,
            )?
            .as_string()
            .map(|value| value.to_string())
    }
}

fn main() -> JSResult<()> {
    let mut runtime = HostRuntime::new()?;
    runtime.start()?;

    for _ in 0..4 {
        runtime.tick()?;
    }

    let summary = runtime.summary()?;
    if summary != "kedojs:public-api:kedo://app.js:ready" {
        println!("kedo integration debug: {}", runtime.debug_state()?);
    }
    assert_eq!(summary, "kedojs:public-api:kedo://app.js:ready");
    println!("kedo integration: {summary}");
    Ok(())
}
