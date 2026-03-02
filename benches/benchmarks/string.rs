use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{JSString, JSStringProctected};

fn bench_string_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_create");

    group.bench_function("short_8b", |b| {
        b.iter(|| {
            let s = JSString::from("hello!!!");
            black_box(s);
        });
    });

    group.bench_function("medium_128b", |b| {
        let input = "x".repeat(128);
        b.iter(|| {
            let s = JSString::from(input.as_str());
            black_box(s);
        });
    });

    group.bench_function("long_4kb", |b| {
        let input = "y".repeat(4096);
        b.iter(|| {
            let s = JSString::from(input.as_str());
            black_box(s);
        });
    });

    group.bench_function("large_64kb", |b| {
        let input = "z".repeat(65536);
        b.iter(|| {
            let s = JSString::from(input.as_str());
            black_box(s);
        });
    });

    group.finish();
}

fn bench_string_equality(c: &mut Criterion) {
    let a = JSString::from("hello world benchmark test string");
    let b = JSString::from("hello world benchmark test string");
    let c_str = JSString::from("different string entirely");

    c.bench_function("string_equality_same", |b_iter| {
        b_iter.iter(|| {
            black_box(a == b);
        });
    });

    c.bench_function("string_equality_different", |b_iter| {
        b_iter.iter(|| {
            black_box(a == c_str);
        });
    });
}

fn bench_string_len(c: &mut Criterion) {
    let s = JSString::from("hello world benchmark test string");

    c.bench_function("string_len", |b| {
        b.iter(|| {
            black_box(s.len());
        });
    });
}

fn bench_string_to_rust(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_to_rust");

    group.bench_function("short", |b| {
        let s = JSString::from("hello");
        b.iter(|| {
            let rust_str: String = s.to_string();
            black_box(rust_str);
        });
    });

    group.bench_function("long_4kb", |b| {
        let input = "a".repeat(4096);
        let s = JSString::from(input.as_str());
        b.iter(|| {
            let rust_str: String = s.to_string();
            black_box(rust_str);
        });
    });

    group.finish();
}

fn bench_string_protected_lifecycle(c: &mut Criterion) {
    c.bench_function("string_protected_create_drop", |b| {
        b.iter(|| {
            let s = JSStringProctected::from("protected string lifecycle");
            black_box(&s);
            // s is not auto-dropped; it stays alive until explicitly released
            // This measures the creation overhead
        });
    });
}

fn bench_string_compare_with_str(c: &mut Criterion) {
    let js_str = JSString::from("benchmark comparison string");

    c.bench_function("string_compare_with_str", |b| {
        b.iter(|| {
            black_box(js_str == "benchmark comparison string");
        });
    });

    c.bench_function("string_compare_with_str_mismatch", |b| {
        b.iter(|| {
            black_box(js_str == "something completely different here");
        });
    });
}

criterion_group!(
    benches,
    bench_string_create,
    bench_string_equality,
    bench_string_len,
    bench_string_to_rust,
    bench_string_protected_lifecycle,
    bench_string_compare_with_str,
);
