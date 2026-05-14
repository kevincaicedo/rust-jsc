use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{
    callback, CallbackContext, JSContext, JSFunction, JSResult, JSValue, Promise,
};

#[callback]
fn host_add(_ctx: CallbackContext, left: i32, right: i32) -> JSResult<i32> {
    Ok(left + right)
}

fn bench_minimal_runtime_startup(c: &mut Criterion) {
    c.bench_function("embedding_minimal_runtime_startup", |b| {
        b.iter(|| {
            let ctx = JSContext::new();
            let add = JSFunction::callback(&ctx, Some("hostAdd"), Some(host_add));
            let add_value: JSValue = add.into();
            ctx.global_object()
                .set_property("hostAdd", &add_value, Default::default())
                .unwrap();
            let result = ctx.evaluate_script("hostAdd(20, 22)", None).unwrap();
            black_box(result);
        });
    });
}

fn bench_host_promise_completion_checkpoint(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("embedding_host_promise_checkpoint", |b| {
        b.iter(|| {
            let (promise, resolver) = Promise::new_pending(&ctx).unwrap();
            resolver
                .resolve(None, &[JSValue::string(&ctx, "ready")])
                .unwrap();
            ctx.run_deferred_work();
            ctx.run_microtasks();
            black_box(promise);
        });
    });
}

criterion_group!(
    benches,
    bench_minimal_runtime_startup,
    bench_host_promise_completion_checkpoint,
);
