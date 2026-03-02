use rust_jsc::{
    callback, JSArray, JSArrayBuffer, JSClass, JSContext, JSError, JSFunction, JSObject, JSPromise,
    JSRegExp, JSResult, JSTypedArray, JSTypedArrayType, JSValue, PropertyDescriptorBuilder,
};
use std::time::Instant;

// ─── Callbacks ──────────────────────────────────────────────────────────────
#[callback]
fn console_log(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let parts: Vec<String> = arguments
        .iter()
        .map(|a| {
            a.as_string()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("{:?}", a.as_json_string(0)))
        })
        .collect();
    println!("  [JS] {}", parts.join(" "));
    Ok(JSValue::undefined(&ctx))
}

fn setup_console(ctx: &JSContext) {
    let global = ctx.global_object();
    let console = JSObject::new(ctx);
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let log_fn = JSFunction::callback(ctx, Some("log"), Some(console_log));
    console.set_property("log", &log_fn, attrs).unwrap();
    console.set_property("info", &log_fn, attrs).unwrap();
    console.set_property("warn", &log_fn, attrs).unwrap();
    console.set_property("error", &log_fn, attrs).unwrap();

    global.set_property("console", &console, attrs).unwrap();
}

fn timed<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    println!("  [Stress] {:<30} took {:.2}ms", name, elapsed.as_secs_f64() * 1000.0);
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
        ).unwrap();
    });
}

fn scenario_json_processing() {
    timed("json_processing_pipeline", || {
        let ctx = JSContext::new();
        let mut items = Vec::new();
        for i in 0..500 {
            items.push(format!(
                r#"{{"id":{},"name":"user_{}","active":{},"score":{}}}"#,
                i, i, if i % 3 != 0 { "true" } else { "false" }, i * 10
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
        let ta = JSTypedArray::with_bytes(&ctx, data.as_mut_slice(), JSTypedArrayType::Uint8Array).unwrap();
        
        let global = ctx.global_object();
        let attrs = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        global.set_property("inputArray", &ta.into(), attrs).unwrap();

        ctx.evaluate_script(
            r#"
            const output = new Uint8Array(inputArray.length);
            for (let i = 0; i < inputArray.length; i++) {
                output[i] = (inputArray[i] * 2) % 256;
            }
            "#,
            None,
        ).unwrap();
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
            obj.set_property(format!("prop_{}", i), &val, attrs).unwrap();
        }
        for i in 0..10_000 {
            let _ = obj.get_property(format!("prop_{}", i).as_str()).unwrap();
        }
    });
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
            if cycle % 3 == 0 { ctx.garbage_collect(); }
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
    scenario_property_storm();
    scenario_memory_churn();
    
    println!("=== Stress tests complete ===");
}
