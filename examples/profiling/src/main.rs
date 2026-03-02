//! DHAT Heap Profiling Harness for rust-jsc
//!
//! This binary instruments the global allocator with dhat to track all heap
//! allocations. When it exits, it writes `dhat-heap.json` to the current
//! directory, which can be visualized at:
//!   https://nnethercote.github.io/dh_view/dh_view.html
//!
//! Usage:
//!   cargo run --manifest-path profiling/Cargo.toml --release
//!
//! The profiler runs through several representative workloads to capture
//! allocation patterns.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use rust_jsc::{
    callback, JSArray, JSContext, JSFunction,
    JSObject, JSResult, JSTypedArray, JSTypedArrayType, JSValue,
    PropertyDescriptorBuilder,
};

#[callback]
fn noop(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _arguments: &[JSValue],
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

fn scenario_context_churn() {
    println!("[Profile] Context creation/destruction × 100");
    for _ in 0..100 {
        let ctx = JSContext::new();
        ctx.evaluate_script("1 + 1", None).ok();
        drop(ctx);
    }
}

fn scenario_value_creation() {
    println!("[Profile] Value creation × 50K");
    let ctx = JSContext::new();
    for i in 0..50_000 {
        match i % 4 {
            0 => {
                let _ = JSValue::number(&ctx, i as f64);
            }
            1 => {
                let _ = JSValue::string(&ctx, "test_string_value");
            }
            2 => {
                let _ = JSValue::boolean(&ctx, i % 2 == 0);
            }
            _ => {
                let _ = JSValue::null(&ctx);
            }
        }
    }
    ctx.garbage_collect();
}

fn scenario_object_properties() {
    println!("[Profile] Object property set/get × 10K");
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
        let _ = obj.get_property(format!("prop_{}", i).as_str());
    }
    ctx.garbage_collect();
}

fn scenario_typed_arrays() {
    println!("[Profile] TypedArray creation × 100 (64KB each)");
    let ctx = JSContext::new();
    let mut data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();

    for _ in 0..100 {
        let ta =
            JSTypedArray::with_bytes(&ctx, data.as_mut_slice(), JSTypedArrayType::Uint8Array).unwrap();
        let _ = ta.as_vec::<u8>();
    }
    ctx.garbage_collect();
}

fn scenario_script_evaluation() {
    println!("[Profile] Complex script evaluation × 50");
    let ctx = JSContext::new();

    let script = r#"
        (function() {
            let sum = 0;
            const data = [];
            for (let i = 0; i < 1000; i++) {
                data.push({ id: i, name: 'item_' + i, value: Math.sqrt(i) });
                sum += data[i].value;
            }
            const filtered = data.filter(d => d.id % 2 === 0);
            const mapped = filtered.map(d => d.value * 2);
            return mapped.reduce((a, b) => a + b, 0);
        })()
    "#;

    for _ in 0..50 {
        ctx.evaluate_script(script, None).unwrap();
    }
    ctx.garbage_collect();
}

fn scenario_function_callbacks() {
    println!("[Profile] Function callback calls × 10K");
    let ctx = JSContext::new();
    let global = ctx.global_object();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let f = JSFunction::callback(&ctx, Some("noop"), Some(noop));
    global.set_property("noop", &f, attrs).unwrap();

    ctx.evaluate_script(
        "for (let i = 0; i < 10000; i++) noop();",
        None,
    )
    .unwrap();
    ctx.garbage_collect();
}

fn scenario_gc_pressure() {
    println!("[Profile] GC pressure: allocate + collect × 30 cycles");
    let ctx = JSContext::new();

    for cycle in 0..30 {
        let script = format!(
            r#"
            var batch = [];
            for (var i = 0; i < 5000; i++) {{
                batch.push({{ id: i, data: 'x'.repeat(50), cycle: {} }});
            }}
            batch = null;
            "#,
            cycle
        );
        ctx.evaluate_script(&script, None).unwrap();

        if cycle % 3 == 0 {
            ctx.garbage_collect();
        }
    }
    ctx.garbage_collect();
}

fn scenario_array_operations() {
    println!("[Profile] Array push + read × 20K");
    let ctx = JSContext::new();
    let arr = JSArray::new_array(&ctx, &[]).unwrap();

    for i in 0..20_000 {
        arr.push(&JSValue::number(&ctx, i as f64)).unwrap();
    }

    for i in (0..20_000).step_by(100) {
        let _ = arr.get(i);
    }
    ctx.garbage_collect();
}

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("=== DHAT Heap Profiling Harness ===\n");

    scenario_context_churn();
    scenario_value_creation();
    scenario_object_properties();
    scenario_typed_arrays();
    scenario_script_evaluation();
    scenario_function_callbacks();
    scenario_gc_pressure();
    scenario_array_operations();

    println!("\n=== Profiling complete. See dhat-heap.json ===");
    // _profiler is dropped here, writing dhat-heap.json
}
