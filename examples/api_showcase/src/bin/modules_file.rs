use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rust_jsc::{JSContext, JSResult, ModuleLoader};

const WASM_ANSWER_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
    0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, 0x61, 0x6e, 0x73, 0x77, 0x65,
    0x72, 0x00, 0x00, 0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
];

fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "rust-jsc-api-showcase-modules-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn write(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
}

fn write_bytes(path: &Path, source: &[u8]) {
    fs::write(path, source).unwrap();
}

fn wait_for_string(ctx: &JSContext, name: &str) -> Option<String> {
    for _ in 0..100 {
        ctx.run_deferred_work();
        ctx.run_microtasks();
        let value = ctx
            .evaluate_script(&format!("globalThis.{name}"), None)
            .unwrap();
        if value.is_string() {
            return Some(value.as_string().unwrap().to_string());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

fn wait_for_number(ctx: &JSContext, name: &str, expected: f64) -> bool {
    for _ in 0..100 {
        ctx.run_deferred_work();
        ctx.run_microtasks();
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

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    ctx.set_module_loader(ModuleLoader::file_system());

    let wasm_available = ctx
        .evaluate_script("typeof WebAssembly !== 'undefined'", None)?
        .as_boolean();

    let dir = temp_dir();
    write(
        &dir.join("math.js"),
        "export const label = 'file-js'; export const add = (left, right) => left + right;",
    );
    write(&dir.join("settings.json"), r#"{"name":"file-json"}"#);
    write_bytes(&dir.join("answer.wasm"), WASM_ANSWER_MODULE);
    write(
        &dir.join("main.js"),
        r#"
        import { label, add } from './math.js';
        import settings from './settings.json' with { type: 'json' };

        globalThis.fileSummary = `${label}:${settings.name}:${add(20, 22)}:${import.meta.url.startsWith('file://')}`;

        if (typeof WebAssembly !== 'undefined') {
            import('./answer.wasm').then(
                module => { globalThis.wasmAnswer = module.answer(); },
                error => { globalThis.wasmError = String(error && error.message || error); },
            );
        }
        "#,
    );

    let promise = ctx.evaluate_module(dir.join("main.js").to_string_lossy().as_ref())?;
    ctx.global_object().set_property(
        "__fileModulePromise",
        &promise,
        Default::default(),
    )?;
    ctx.evaluate_script(
        r#"
        __fileModulePromise.catch((error) => {
            globalThis.fileModuleError = String(error && (error.stack || error));
        });
        "#,
        None,
    )?;

    let summary = wait_for_string(&ctx, "fileSummary")
        .expect("file module did not set fileSummary");
    assert_eq!(summary, "file-js:file-json:42:true");

    if wasm_available {
        assert!(wait_for_number(&ctx, "wasmAnswer", 42.0));
        let wasm_error = ctx.evaluate_script("globalThis.wasmError", None)?;
        assert!(
            wasm_error.is_undefined(),
            "wasm import failed: {wasm_error:?}"
        );
    }

    let module_error = ctx.evaluate_script("globalThis.fileModuleError", None)?;
    assert!(
        module_error.is_undefined(),
        "file module failed: {module_error:?}"
    );

    let _ = fs::remove_dir_all(dir);
    println!("modules file: {summary} wasm={wasm_available}");
    Ok(())
}
