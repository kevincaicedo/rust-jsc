use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use rust_jsc::{
    JSContext, JSObject, JSValue, PropertyDescriptor, PropertyDescriptorBuilder,
};

fn bench_object_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("object_create", |b| {
        b.iter(|| {
            black_box(JSObject::new(&ctx));
        });
    });
}

fn bench_object_set_property(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let mut group = c.benchmark_group("object_set_property");

    for count in [1, 10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &n| {
            b.iter(|| {
                let obj = JSObject::new(&ctx);
                for i in 0..n {
                    let val = JSValue::number(&ctx, i as f64);
                    obj.set_property(format!("prop_{}", i), &val, attrs)
                        .unwrap();
                }
                black_box(&obj);
            });
        });
    }
    group.finish();
}

fn bench_object_get_property(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    // Pre-populate object
    let obj = JSObject::new(&ctx);
    for i in 0..100 {
        let val = JSValue::number(&ctx, i as f64);
        obj.set_property(format!("prop_{}", i), &val, attrs)
            .unwrap();
    }

    c.bench_function("object_get_property_hit", |b| {
        b.iter(|| {
            let val = obj.get_property(black_box("prop_50")).unwrap();
            black_box(val);
        });
    });

    c.bench_function("object_get_property_miss", |b| {
        b.iter(|| {
            let val = obj.get_property(black_box("nonexistent"));
            let _ = black_box(val);
        });
    });
}

fn bench_object_has_property(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let obj = JSObject::new(&ctx);
    for i in 0..50 {
        let val = JSValue::number(&ctx, i as f64);
        obj.set_property(format!("key_{}", i), &val, attrs).unwrap();
    }

    c.bench_function("object_has_property_true", |b| {
        b.iter(|| {
            black_box(obj.has_property(black_box("key_25")));
        });
    });

    c.bench_function("object_has_property_false", |b| {
        b.iter(|| {
            black_box(obj.has_property(black_box("missing")));
        });
    });
}

fn bench_object_delete_property(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    c.bench_function("object_delete_property", |b| {
        b.iter(|| {
            let obj = JSObject::new(&ctx);
            for i in 0..10 {
                let val = JSValue::number(&ctx, i as f64);
                obj.set_property(format!("del_{}", i), &val, attrs).unwrap();
            }
            for i in 0..10 {
                obj.delete_property(format!("del_{}", i)).unwrap();
            }
            black_box(&obj);
        });
    });
}

fn bench_object_property_names(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let obj = JSObject::new(&ctx);
    for i in 0..100 {
        let val = JSValue::number(&ctx, i as f64);
        obj.set_property(format!("prop_{}", i), &val, attrs)
            .unwrap();
    }

    c.bench_function("object_property_names_100", |b| {
        b.iter(|| {
            let names: Vec<_> = obj.get_property_names().collect();
            black_box(names);
        });
    });
}

fn bench_object_prototype_chain(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    c.bench_function("object_prototype_set_lookup", |b| {
        b.iter(|| {
            let proto = JSObject::new(&ctx);
            proto
                .set_property("inherited", &JSValue::number(&ctx, 42.0), attrs)
                .unwrap();

            let child = JSObject::new(&ctx);
            child.set_prototype(&proto);

            // Access inherited property via script
            let global = ctx.global_object();
            global.set_property("__bench_child", &child, attrs).unwrap();
            let result = ctx
                .evaluate_script("__bench_child.inherited", None)
                .unwrap();
            black_box(result);
        });
    });
}

fn bench_object_deep_nesting(c: &mut Criterion) {
    let ctx = JSContext::new();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    c.bench_function("object_deep_nesting_50", |b| {
        b.iter(|| {
            let mut current = JSObject::new(&ctx);
            for i in 0..50 {
                let child = JSObject::new(&ctx);
                child
                    .set_property("value", &JSValue::number(&ctx, i as f64), attrs)
                    .unwrap();
                current.set_property("child", &child, attrs).unwrap();
                current = child;
            }
            black_box(&current);
        });
    });
}

fn bench_object_set_by_key(c: &mut Criterion) {
    let ctx = JSContext::new();
    let desc = PropertyDescriptor::default();

    c.bench_function("object_set_by_jsvalue_key", |b| {
        b.iter(|| {
            let obj = JSObject::new(&ctx);
            for i in 0..50 {
                let key = JSValue::string(&ctx, format!("k_{}", i));
                let val = JSValue::number(&ctx, i as f64);
                obj.set(&key, &val, desc).unwrap();
            }
            black_box(&obj);
        });
    });
}

fn bench_object_private_data(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("object_set_private_data", |b| {
        b.iter(|| {
            let obj = JSObject::new(&ctx);
            unsafe { obj.set_private_data(42i32) };
            black_box(&obj);
        });
    });

    let obj = JSObject::new(&ctx);
    unsafe { obj.set_private_data(42i32) };

    c.bench_function("object_get_private_data_hit", |b| {
        b.iter(|| {
            let data = obj.get_private_data::<i32>();
            black_box(data);
        });
    });

    c.bench_function("object_get_private_data_miss", |b| {
        b.iter(|| {
            let data = obj.get_private_data::<String>();
            black_box(data);
        });
    });
}

criterion_group!(
    benches,
    bench_object_create,
    bench_object_set_property,
    bench_object_get_property,
    bench_object_has_property,
    bench_object_delete_property,
    bench_object_property_names,
    bench_object_prototype_chain,
    bench_object_deep_nesting,
    bench_object_set_by_key,
    bench_object_private_data,
);
