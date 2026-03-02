use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use rust_jsc::{JSContext, JSObject, JSValue, PropertyDescriptorBuilder};

fn bench_gc_after_object_burst(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("gc_after_object_burst");

    for count in [1000, 10000, 50000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &n| {
                let script = format!(
                    "var a = []; for (var i = 0; i < {}; i++) a.push({{x: i, y: i*2}}); a = null;",
                    n
                );
                b.iter(|| {
                    ctx.evaluate_script(&script, None).unwrap();
                    ctx.garbage_collect();
                });
            },
        );
    }
    group.finish();
}

fn bench_gc_with_protected_values(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("gc_with_protected_values");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &n| {
                b.iter(|| {
                    let values: Vec<JSValue> = (0..n)
                        .map(|i| {
                            let v = JSValue::number(&ctx, i as f64);
                            v.protect();
                            v
                        })
                        .collect();

                    // Create garbage alongside protected values
                    ctx.evaluate_script(
                        "var g = []; for (var i = 0; i < 1000; i++) g.push({z: i}); g = null;",
                        None,
                    )
                    .unwrap();

                    ctx.garbage_collect();

                    // Unprotect after GC
                    for v in &values {
                        v.unprotect();
                    }
                    black_box(&values);
                });
            },
        );
    }
    group.finish();
}

fn bench_gc_reclamation_rate(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("gc_reclamation_measurement", |b| {
        b.iter(|| {
            // Create significant garbage
            ctx.evaluate_script(
                r#"
                var data = [];
                for (var i = 0; i < 10000; i++) {
                    data.push({
                        id: i,
                        name: 'item_' + i,
                        nested: { x: i, y: i * 2 }
                    });
                }
                data = null;
                "#,
                None,
            )
            .unwrap();

            let _before = ctx.get_memory_usage();
            ctx.garbage_collect();
            let _after = ctx.get_memory_usage();
            black_box((_before, _after));
        });
    });
}

fn bench_gc_under_pressure(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("gc_under_continuous_pressure", |b| {
        b.iter(|| {
            for cycle in 0..10 {
                // Allocate
                let script = format!(
                    "var batch_{} = []; for (var i = 0; i < 1000; i++) batch_{}.push({{v: i}}); batch_{} = null;",
                    cycle, cycle, cycle
                );
                ctx.evaluate_script(&script, None).unwrap();

                // Periodic GC
                if cycle % 3 == 0 {
                    ctx.garbage_collect();
                }
            }
            // Final GC
            ctx.garbage_collect();
        });
    });
}

fn bench_gc_native_objects(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    c.bench_function("gc_native_object_churn", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let obj = JSObject::new(&ctx);
                obj.set_property("x", &JSValue::number(&ctx, 42.0), attrs)
                    .unwrap();
                // obj goes out of scope here — becomes garbage
            }
            ctx.garbage_collect();
        });
    });
}

criterion_group!(
    benches,
    bench_gc_after_object_burst,
    bench_gc_with_protected_values,
    bench_gc_reclamation_rate,
    bench_gc_under_pressure,
    bench_gc_native_objects,
);
