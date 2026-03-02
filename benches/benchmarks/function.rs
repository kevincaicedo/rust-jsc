use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{
    callback, JSContext, JSFunction, JSObject, JSResult, JSValue,
    PropertyDescriptorBuilder,
};

#[callback]
fn noop_callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _arguments: &[JSValue],
) -> JSResult<JSValue> {
    Ok(JSValue::undefined(&ctx))
}

#[callback]
fn add_callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let a = arguments.get(0).unwrap().as_number().unwrap();
    let b = arguments.get(1).unwrap().as_number().unwrap();
    Ok(JSValue::number(&ctx, a + b))
}

#[callback]
fn string_return_callback(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _arguments: &[JSValue],
) -> JSResult<JSValue> {
    Ok(JSValue::string(&ctx, "benchmark result"))
}

fn bench_function_callback_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("function_callback_create", |b| {
        b.iter(|| {
            let f = JSFunction::callback(&ctx, Some("test"), Some(noop_callback));
            black_box(f);
        });
    });
}

fn bench_function_call_noop(c: &mut Criterion) {
    let ctx = JSContext::new();
    let f = JSFunction::callback(&ctx, Some("noop"), Some(noop_callback));

    c.bench_function("function_call_noop", |b| {
        b.iter(|| {
            let result = f.call(None, &[]);
            black_box(result.unwrap());
        });
    });
}

fn bench_function_call_with_args(c: &mut Criterion) {
    let ctx = JSContext::new();
    let f = JSFunction::callback(&ctx, Some("add"), Some(add_callback));

    let mut group = c.benchmark_group("function_call_with_args");

    group.bench_function("2_args", |b| {
        let args = vec![JSValue::number(&ctx, 10.0), JSValue::number(&ctx, 20.0)];
        b.iter(|| {
            let result = f.call(None, &args);
            black_box(result.unwrap());
        });
    });

    group.finish();
}

fn bench_function_call_return_value(c: &mut Criterion) {
    let ctx = JSContext::new();
    let f = JSFunction::callback(&ctx, Some("strret"), Some(string_return_callback));

    c.bench_function("function_call_return_string", |b| {
        b.iter(|| {
            let result = f.call(None, &[]).unwrap();
            black_box(result.as_string().unwrap());
        });
    });
}

fn bench_function_js_to_rust_roundtrip(c: &mut Criterion) {
    let ctx = JSContext::new();
    let global = ctx.global_object();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let f = JSFunction::callback(&ctx, Some("add"), Some(add_callback));
    global.set_property("rustAdd", &f, attrs).unwrap();

    c.bench_function("function_js_to_rust_roundtrip", |b| {
        b.iter(|| {
            let result = ctx
                .evaluate_script(black_box("rustAdd(3, 4)"), None)
                .unwrap();
            black_box(result);
        });
    });
}

fn bench_function_call_from_js_repeated(c: &mut Criterion) {
    let ctx = JSContext::new();
    let global = ctx.global_object();
    let attrs = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();

    let f = JSFunction::callback(&ctx, Some("noop"), Some(noop_callback));
    global.set_property("rustNoop", &f, attrs).unwrap();

    c.bench_function("function_call_from_js_100x", |b| {
        b.iter(|| {
            let result = ctx
                .evaluate_script(
                    black_box("for (let i = 0; i < 100; i++) rustNoop(); true"),
                    None,
                )
                .unwrap();
            black_box(result);
        });
    });
}

fn bench_function_constructor(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("function_call_as_constructor_via_js", |b| {
        b.iter(|| {
            let result = ctx
                .evaluate_script(
                    black_box(
                        r#"
                        function Point(x, y) { this.x = x; this.y = y; }
                        new Point(1, 2);
                    "#,
                    ),
                    None,
                )
                .unwrap();
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_function_callback_create,
    bench_function_call_noop,
    bench_function_call_with_args,
    bench_function_call_return_value,
    bench_function_js_to_rust_roundtrip,
    bench_function_call_from_js_repeated,
    bench_function_constructor,
);
