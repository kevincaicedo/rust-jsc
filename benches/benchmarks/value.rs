use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{JSContext, JSValue};

fn bench_value_create_number(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("value_create_number", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(JSValue::number(&ctx, i as f64));
            }
        });
    });
}

fn bench_value_create_boolean(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("value_create_boolean", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(JSValue::boolean(&ctx, true));
                black_box(JSValue::boolean(&ctx, false));
            }
        });
    });
}

fn bench_value_create_null_undefined(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("value_create_null", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(JSValue::null(&ctx));
            }
        });
    });

    c.bench_function("value_create_undefined", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(JSValue::undefined(&ctx));
            }
        });
    });
}

fn bench_value_create_string(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("value_create_string");

    group.bench_function("short_8b", |b| {
        b.iter(|| {
            black_box(JSValue::string(&ctx, "hello!!!"));
        });
    });

    group.bench_function("medium_128b", |b| {
        let s = "x".repeat(128);
        b.iter(|| {
            black_box(JSValue::string(&ctx, s.as_str()));
        });
    });

    group.bench_function("long_4kb", |b| {
        let s = "y".repeat(4096);
        b.iter(|| {
            black_box(JSValue::string(&ctx, s.as_str()));
        });
    });

    group.bench_function("large_64kb", |b| {
        let s = "z".repeat(65536);
        b.iter(|| {
            black_box(JSValue::string(&ctx, s.as_str()));
        });
    });

    group.finish();
}

fn bench_value_create_symbol(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("value_create_symbol", |b| {
        b.iter(|| {
            black_box(JSValue::symbol(&ctx, "my_symbol"));
        });
    });
}

fn bench_value_create_from_json(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("value_from_json");

    group.bench_function("simple_object", |b| {
        b.iter(|| {
            black_box(JSValue::from_json(&ctx, r#"{"key": "value", "num": 42}"#));
        });
    });

    let medium_json = format!(
        "[{}]",
        (0..50)
            .map(|i| format!(r#"{{"id": {}, "name": "item_{}", "active": true}}"#, i, i))
            .collect::<Vec<_>>()
            .join(",")
    );
    group.bench_function("medium_array_50", |b| {
        b.iter(|| {
            black_box(JSValue::from_json(&ctx, medium_json.as_str()));
        });
    });

    let large_json = format!(
        "[{}]",
        (0..500)
            .map(|i| format!(
                r#"{{"id": {}, "name": "item_{}", "nested": {{"x": {}, "y": {}}}}}"#,
                i,
                i,
                i * 10,
                i * 20
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    group.bench_function("large_array_500", |b| {
        b.iter(|| {
            black_box(JSValue::from_json(&ctx, large_json.as_str()));
        });
    });

    group.finish();
}

fn bench_value_as_number(c: &mut Criterion) {
    let ctx = JSContext::new();
    let val = JSValue::number(&ctx, 42.0);

    c.bench_function("value_as_number", |b| {
        b.iter(|| {
            black_box(val.as_number().unwrap());
        });
    });
}

fn bench_value_as_string(c: &mut Criterion) {
    let ctx = JSContext::new();
    let val = JSValue::string(&ctx, "hello world test string for benchmark");

    c.bench_function("value_as_string", |b| {
        b.iter(|| {
            black_box(val.as_string().unwrap());
        });
    });
}

fn bench_value_as_boolean(c: &mut Criterion) {
    let ctx = JSContext::new();
    let val = JSValue::boolean(&ctx, true);

    c.bench_function("value_as_boolean", |b| {
        b.iter(|| {
            black_box(val.as_boolean());
        });
    });
}

fn bench_value_type_check(c: &mut Criterion) {
    let ctx = JSContext::new();
    let num = JSValue::number(&ctx, 42.0);
    let str_val = JSValue::string(&ctx, "hello");
    let bool_val = JSValue::boolean(&ctx, true);
    let null_val = JSValue::null(&ctx);
    let undef_val = JSValue::undefined(&ctx);

    c.bench_function("value_type_check_battery", |b| {
        b.iter(|| {
            black_box(num.is_number());
            black_box(num.is_string());
            black_box(str_val.is_string());
            black_box(str_val.is_number());
            black_box(bool_val.is_boolean());
            black_box(null_val.is_null());
            black_box(undef_val.is_undefined());
        });
    });
}

fn bench_value_protect_unprotect(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("value_protect_unprotect", |b| {
        b.iter(|| {
            let val = JSValue::number(&ctx, 99.0);
            val.protect();
            val.unprotect();
        });
    });
}

fn bench_value_json_roundtrip(c: &mut Criterion) {
    let ctx = JSContext::new();
    let json_str = r#"{"name": "test", "values": [1,2,3,4,5], "nested": {"key": "val"}}"#;

    c.bench_function("value_json_roundtrip", |b| {
        b.iter(|| {
            let val = JSValue::from_json(&ctx, json_str);
            let back = val.as_json_string(0).unwrap();
            black_box(back);
        });
    });
}

fn bench_value_equality(c: &mut Criterion) {
    let ctx = JSContext::new();
    let a = JSValue::number(&ctx, 42.0);
    let b = JSValue::number(&ctx, 42.0);
    let c_val = JSValue::string(&ctx, "42");

    c.bench_function("value_equal_same_type", |b_iter| {
        b_iter.iter(|| {
            black_box(a.is_equal(&b).unwrap());
        });
    });

    c.bench_function("value_equal_coercion", |b_iter| {
        b_iter.iter(|| {
            black_box(a.is_equal(&c_val).unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_value_create_number,
    bench_value_create_boolean,
    bench_value_create_null_undefined,
    bench_value_create_string,
    bench_value_create_symbol,
    bench_value_create_from_json,
    bench_value_as_number,
    bench_value_as_string,
    bench_value_as_boolean,
    bench_value_type_check,
    bench_value_protect_unprotect,
    bench_value_json_roundtrip,
    bench_value_equality,
);
