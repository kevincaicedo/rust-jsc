use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use rust_jsc::JSContext;

fn bench_context_create(c: &mut Criterion) {
    c.bench_function("context_create", |b| {
        b.iter(|| {
            let ctx = black_box(JSContext::new());
            drop(ctx);
        });
    });
}

fn bench_evaluate_function_def_and_call(c: &mut Criterion) {
    let ctx = JSContext::new();
    c.bench_function("evaluate_function_def_and_call", |b| {
        b.iter(|| {
            let result = ctx.evaluate_script(
                black_box("(function add(a, b) { return a + b; })(3, 4)"),
                None,
            );
            black_box(result.unwrap());
        });
    });
}

fn bench_evaluate_complex_script(c: &mut Criterion) {
    let ctx = JSContext::new();
    let script = r#"
        (function() {
            let sum = 0;
            for (let i = 0; i < 1000; i++) {
                sum += i;
                let obj = { x: i, y: i * 2, z: i * 3 };
                sum += obj.x + obj.y + obj.z;
            }
            const arr = Array.from({length: 100}, (_, i) => ({
                id: i,
                name: 'item' + i,
                nested: { value: i * i }
            }));
            const filtered = arr.filter(x => x.id % 2 === 0);
            const mapped = filtered.map(x => x.nested.value);
            const reduced = mapped.reduce((a, b) => a + b, 0);
            return sum + reduced;
        })()
    "#;

    c.bench_function("evaluate_complex_script", |b| {
        b.iter(|| {
            let result = ctx.evaluate_script(black_box(script), None);
            black_box(result.unwrap());
        });
    });
}

fn bench_evaluate_json_parse(c: &mut Criterion) {
    let ctx = JSContext::new();

    // Generate a ~10KB JSON string
    let mut json_items: Vec<String> = Vec::new();
    for i in 0..100 {
        json_items.push(format!(
            r#"{{"id": {i}, "name": "item_{i}", "value": {val}, "tags": ["a", "b", "c"], "nested": {{"x": {x}, "y": {y}}}}}"#,
            i = i, val = i as f64 * 1.5, x = i * 10, y = i * 20
        ));
    }
    let json_array = format!("[{}]", json_items.join(","));
    let script = format!("JSON.parse('{}')", json_array.replace('\'', "\\'"));

    c.bench_function("evaluate_json_parse_10kb", |b| {
        b.iter(|| {
            let result = ctx.evaluate_script(black_box(&script), None);
            black_box(result.unwrap());
        });
    });
}

fn bench_check_syntax(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("check_syntax_valid", |b| {
        b.iter(|| {
            let result =
                ctx.check_syntax(black_box("function foo(x) { return x * 2; }"), 0);
            black_box(result.unwrap());
        });
    });

    c.bench_function("check_syntax_invalid", |b| {
        b.iter(|| {
            let result = ctx.check_syntax(black_box("function {{{ invalid"), 0);
            let _ = black_box(result);
        });
    });
}

fn bench_gc_manual(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("gc_after_allocation", |b| {
        b.iter(|| {
            // Create some objects to give GC work to do
            ctx.evaluate_script(
                "var arr = []; for (var i = 0; i < 1000; i++) arr.push({x: i}); arr = null;",
                None,
            )
            .unwrap();
            ctx.garbage_collect();
        });
    });
}

fn bench_evaluate_scaling(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("evaluate_loop_scaling");
    for iterations in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(iterations),
            iterations,
            |b, &iters| {
                let script = format!(
                    "(function() {{ var s = 0; for (var i = 0; i < {}; i++) s += i; return s; }})()",
                    iters
                );
                b.iter(|| {
                    let result = ctx.evaluate_script(black_box(&script), None);
                    black_box(result.unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_context_shared_data(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("context_set_shared_data", |b| {
        b.iter(|| {
            let local_ctx = JSContext::new();
            local_ctx.set_shared_data(42i32);
            black_box(&local_ctx);
        });
    });

    ctx.set_shared_data(42i32);

    c.bench_function("context_get_shared_data_hit", |b| {
        b.iter(|| {
            let data = ctx.get_shared_data::<i32>();
            black_box(data);
        });
    });

    c.bench_function("context_get_shared_data_miss", |b| {
        b.iter(|| {
            let data = ctx.get_shared_data::<String>();
            black_box(data);
        });
    });
}

criterion_group!(
    benches,
    bench_context_create,
    bench_evaluate_function_def_and_call,
    bench_evaluate_complex_script,
    bench_evaluate_json_parse,
    bench_check_syntax,
    bench_gc_manual,
    bench_evaluate_scaling,
    bench_context_shared_data,
);
