use rust_jsc::{
    JSArray, JSContext, JSObject, JSTypedArray, JSTypedArrayType, JSValue,
    ModuleLoader, PropertyDescriptorBuilder,
};
use std::{
    fs,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

fn timed<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    println!(
        "  [Stress] {:<30} took {:.2}ms",
        name,
        elapsed.as_secs_f64() * 1000.0
    );
    result
}

// ═══════════════════════════════════════════════════════════════════════════
//  SCENARIOS (ported from functional tests)
// ═══════════════════════════════════════════════════════════════════════════

fn scenario_fibonacci_stress() {
    timed("fibonacci_recursive_30", || {
        let ctx = JSContext::new();
        ctx.evaluate_script(
            r#"
            function fib(n) {
                if (n <= 1) return n;
                return fib(n - 1) + fib(n - 2);
            }
            fib(30);
            "#,
            None,
        )
        .unwrap();
    });
}

fn scenario_json_processing() {
    timed("json_processing_pipeline", || {
        let ctx = JSContext::new();
        let mut items = Vec::new();
        for i in 0..500 {
            items.push(format!(
                r#"{{"id":{},"name":"user_{}","active":{},"score":{}}}"#,
                i,
                i,
                if i % 3 != 0 { "true" } else { "false" },
                i * 10
            ));
        }
        let json_str = format!("[{}]", items.join(","));

        let script = format!(
            r#"
            const data = JSON.parse('{}');
            const active = data.filter(u => u.active);
            const transformed = active.map(u => ({{
                ...u,
                label: u.name.toUpperCase(),
                doubled: u.score * 2
            }}));
            transformed.reduce((a, b) => a + b.doubled, 0);
            "#,
            json_str.replace('\\', "\\\\").replace('\'', "\\'")
        );
        ctx.evaluate_script(&script, None).unwrap();
    });
}

fn scenario_typed_array_stress() {
    timed("typed_array_data_pipeline", || {
        let ctx = JSContext::new();
        let mut data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
        let ta = JSTypedArray::with_bytes(
            &ctx,
            data.as_mut_slice(),
            JSTypedArrayType::Uint8Array,
        )
        .unwrap();

        let global = ctx.global_object();
        let attrs = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        global
            .set_property("inputArray", &ta.into(), attrs)
            .unwrap();

        ctx.evaluate_script(
            r#"
            const output = new Uint8Array(inputArray.length);
            for (let i = 0; i < inputArray.length; i++) {
                output[i] = (inputArray[i] * 2) % 256;
            }
            "#,
            None,
        )
        .unwrap();
    });
}

fn scenario_array_and_method_helpers() {
    timed("array_method_api_helpers", || {
        let ctx = JSContext::new();
        let array = JSArray::new_array(&ctx, &[]).unwrap();
        for i in 0..2_000 {
            let value = JSValue::number(&ctx, i as f64);
            array.push(&value).unwrap();
        }
        assert_eq!(array.length().unwrap(), 2_000);

        let object = ctx
            .evaluate_script(
                "({ base: 41, add(value) { return this.base + value; } })",
                None,
            )
            .unwrap()
            .as_object()
            .unwrap();
        let result = object
            .call_method("add", &[JSValue::number(&ctx, 1.0)])
            .unwrap();
        assert_eq!(result.as_number().unwrap(), 42.0);
    });
}

fn scenario_module_loader_stress() {
    timed("file_module_loader_real_files", || {
        let ctx = JSContext::new();
        ctx.set_module_loader(ModuleLoader::file_system());

        let dir = temp_dir("rust-jsc-stress-modules");
        fs::write(
            dir.join("config.json"),
            r#"{"name":"stress-json","count":100}"#,
        )
        .unwrap();
        fs::write(
            dir.join("dep.js"),
            "export const label = 'stress-js'; export const plus = value => value + 23;",
        )
        .unwrap();
        fs::write(
            dir.join("main.js"),
            r#"
                import { label, plus } from './dep.js';
                import config from './config.json' with { type: 'json' };
                globalThis.stressModuleSummary = `${label}:${config.name}:${plus(config.count)}`;
            "#,
        )
        .unwrap();

        ctx.evaluate_module(dir.join("main.js").to_string_lossy().as_ref())
            .unwrap();
        for _ in 0..8 {
            ctx.run_deferred_work();
            ctx.run_microtasks();
        }

        let summary = ctx
            .evaluate_script("globalThis.stressModuleSummary", None)
            .unwrap()
            .as_string()
            .unwrap()
            .to_string();
        assert_eq!(summary, "stress-js:stress-json:123");

        let source_url = dir.join("source-entry.js");
        ctx.evaluate_module_from_source(
            r#"
                import { plus } from './dep.js';
                globalThis.stressSourceSummary = plus(19);
            "#,
            source_url.to_string_lossy().as_ref(),
            None,
        )
        .unwrap();
        for _ in 0..8 {
            ctx.run_deferred_work();
            ctx.run_microtasks();
        }
        assert_eq!(
            ctx.evaluate_script("globalThis.stressSourceSummary", None)
                .unwrap()
                .as_number()
                .unwrap(),
            42.0
        );

        let _ = fs::remove_dir_all(dir);
    });
}

fn scenario_property_storm() {
    timed("property_storm_10k", || {
        let ctx = JSContext::new();
        let attrs = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();

        let obj = JSObject::new(&ctx);
        for i in 0..10_000 {
            let val = JSValue::number(&ctx, i as f64);
            obj.set_property(format!("prop_{}", i), &val, attrs)
                .unwrap();
        }
        for i in 0..10_000 {
            let _ = obj.get_property(format!("prop_{}", i).as_str()).unwrap();
        }
    });
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "{prefix}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn scenario_memory_churn() {
    timed("memory_stress_churn", || {
        let ctx = JSContext::new();
        for cycle in 0..10 {
            let script = format!(
                r#"
                var batch = [];
                for (var i = 0; i < 5000; i++) {{
                    batch.push({{ id: i, data: 'x'.repeat(100), nested: {{ a: i }} }});
                }}
                batch = null;
                "#
            );
            ctx.evaluate_script(&script, None).unwrap();
            if cycle % 3 == 0 {
                ctx.garbage_collect();
            }
        }
        ctx.garbage_collect();
    });
}

fn main() {
    println!("=== Rust-JSC Stress Example ===");
    println!("Running complex scenarios for profiling and manual verification...");

    scenario_fibonacci_stress();
    scenario_json_processing();
    scenario_typed_array_stress();
    scenario_array_and_method_helpers();
    scenario_module_loader_stress();
    scenario_property_storm();
    scenario_memory_churn();

    println!("=== Stress tests complete ===");
}
