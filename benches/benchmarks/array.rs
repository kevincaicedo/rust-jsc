use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use rust_jsc::{
    JSArray, JSArrayBuffer, JSContext, JSTypedArray, JSTypedArrayType, JSValue,
};

fn bench_array_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("array_create");

    for count in [1, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &n| {
            let items: Vec<JSValue> =
                (0..n).map(|i| JSValue::number(&ctx, i as f64)).collect();
            b.iter(|| {
                let arr = JSArray::new_array(&ctx, &items).unwrap();
                black_box(arr);
            });
        });
    }
    group.finish();
}

fn bench_array_push(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("array_push_1000", |b| {
        b.iter(|| {
            let arr = JSArray::new_array(&ctx, &[]).unwrap();
            for i in 0..1000 {
                let val = JSValue::number(&ctx, i as f64);
                arr.push(&val).unwrap();
            }
            black_box(&arr);
        });
    });
}

fn bench_array_get_set(c: &mut Criterion) {
    let ctx = JSContext::new();
    let items: Vec<JSValue> = (0..100).map(|i| JSValue::number(&ctx, i as f64)).collect();
    let arr = JSArray::new_array(&ctx, &items).unwrap();

    c.bench_function("array_get_by_index", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(arr.get(i).unwrap());
            }
        });
    });

    c.bench_function("array_set_by_index", |b| {
        b.iter(|| {
            for i in 0..100 {
                arr.set(i, &JSValue::number(&ctx, i as f64 * 2.0)).unwrap();
            }
        });
    });
}

fn bench_typed_array_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("typed_array_create");

    for size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            b.iter(|| {
                let ta = JSTypedArray::new(&ctx, n).unwrap();
                black_box(ta);
            });
        });
    }
    group.finish();
}

fn bench_typed_array_with_bytes(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("typed_array_with_bytes");

    for size in [1024usize, 16384, 65536, 262144].iter() {
        group.bench_with_input(BenchmarkId::new("u8", size), size, |b, &n| {
            b.iter(|| {
                let mut bytes: Vec<u8> = (0..n).map(|i| (i % 256) as u8).collect();
                let ta = JSTypedArray::with_bytes(
                    &ctx,
                    bytes.as_mut_slice(),
                    JSTypedArrayType::Uint8Array,
                )
                .unwrap();
                black_box(ta);
            });
        });
    }
    group.finish();
}

fn bench_typed_array_as_vec(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("typed_array_as_vec");

    for size in [1024usize, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            let mut bytes: Vec<u8> = (0..n).map(|i| (i % 256) as u8).collect();
            let ta = JSTypedArray::with_bytes(
                &ctx,
                bytes.as_mut_slice(),
                JSTypedArrayType::Uint8Array,
            )
            .unwrap();
            b.iter(|| {
                let vec = ta.as_vec::<u8>().unwrap();
                black_box(vec);
            });
        });
    }
    group.finish();
}

fn bench_array_buffer_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    let mut group = c.benchmark_group("array_buffer_create");

    for size in [1024usize, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            b.iter(|| {
                let mut bytes: Vec<u8> = (0..n).map(|i| (i % 256) as u8).collect();
                let buf = JSArrayBuffer::new(&ctx, bytes.as_mut_slice()).unwrap();
                black_box(buf);
            });
        });
    }
    group.finish();
}

fn bench_array_buffer_roundtrip(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("array_buffer_roundtrip_4kb", |b| {
        b.iter(|| {
            let mut data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
            let buf = JSArrayBuffer::new(&ctx, data.as_mut_slice()).unwrap();
            let back = buf.as_vec().unwrap();
            black_box(back);
        });
    });
}

fn bench_typed_array_get_buffer(c: &mut Criterion) {
    let ctx = JSContext::new();
    let mut bytes: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    let ta = JSTypedArray::with_bytes(
        &ctx,
        bytes.as_mut_slice(),
        JSTypedArrayType::Uint8Array,
    )
    .unwrap();

    c.bench_function("typed_array_get_buffer", |b| {
        b.iter(|| {
            let buf = ta.get_buffer().unwrap();
            black_box(buf);
        });
    });
}

criterion_group!(
    benches,
    bench_array_create,
    bench_array_push,
    bench_array_get_set,
    bench_typed_array_create,
    bench_typed_array_with_bytes,
    bench_typed_array_as_vec,
    bench_array_buffer_create,
    bench_array_buffer_roundtrip,
    bench_typed_array_get_buffer,
);
