use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{
    callback, CallbackContext, JSContext, JSFunction, JSObject, JSResult, JSValue,
    Promise,
};

#[callback]
fn capture(ctx: CallbackContext, value: String) -> JSResult<()> {
    ctx.global_object().set_property(
        "__benchPromiseResult",
        &JSValue::string(&ctx, value),
        Default::default(),
    )
}

#[callback]
fn record_unhandled(
    ctx: CallbackContext,
    _promise: JSValue,
    reason: JSValue,
) -> JSResult<()> {
    let message = reason.as_string()?.to_string();
    ctx.global_object().set_property(
        "__benchUnhandledReason",
        &JSValue::string(&ctx, message),
        Default::default(),
    )
}

fn bench_promise_pending_create(c: &mut Criterion) {
    let ctx = JSContext::new();

    c.bench_function("promise_pending_create", |b| {
        b.iter(|| {
            let promise = Promise::new_pending(&ctx).unwrap();
            black_box(promise);
        });
    });
}

fn bench_promise_resolve_microtask(c: &mut Criterion) {
    let ctx = JSContext::new();
    let capture_fn = JSFunction::callback(&ctx, Some("capture"), Some(capture));
    let capture_value: JSValue = capture_fn.into();

    c.bench_function("promise_resolve_microtask", |b| {
        b.iter(|| {
            let (promise, resolver) = Promise::new_pending(&ctx).unwrap();
            promise.then(&[capture_value.clone()]).unwrap();
            resolver
                .resolve(None, &[JSValue::string(&ctx, "ready")])
                .unwrap();
            ctx.run_microtasks();
            black_box(promise);
        });
    });
}

fn bench_unhandled_rejection_handler(c: &mut Criterion) {
    let ctx = JSContext::new();
    let handler =
        JSFunction::callback(&ctx, Some("recordUnhandled"), Some(record_unhandled));
    let handler_object: JSObject = handler.into();
    let _handler_guard = ctx
        .set_unhandled_rejection_handler(&handler_object)
        .unwrap();

    c.bench_function("promise_unhandled_rejection_handler", |b| {
        b.iter(|| {
            let (promise, rejecter) = Promise::new_pending(&ctx).unwrap();
            rejecter
                .reject(None, &[JSValue::string(&ctx, "boom")])
                .unwrap();
            ctx.run_microtasks();
            black_box(promise);
        });
    });
}

criterion_group!(
    benches,
    bench_promise_pending_create,
    bench_promise_resolve_microtask,
    bench_unhandled_rejection_handler,
);
