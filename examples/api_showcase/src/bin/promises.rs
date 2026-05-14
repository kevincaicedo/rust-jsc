use rust_jsc::{
    callback, CallbackContext, JSContext, JSFunction, JSObject, JSResult, JSValue,
    Promise,
};

#[callback]
fn capture(ctx: CallbackContext, value: String) -> JSResult<()> {
    ctx.global_object().set_property(
        "promiseResult",
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
        "unhandledReason",
        &JSValue::string(&ctx, message),
        Default::default(),
    )
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();

    let (promise, resolver) = Promise::new_pending(&ctx)?;
    let capture_fn = JSFunction::callback(&ctx, Some("capture"), Some(capture));
    let capture_value: JSValue = capture_fn.into();
    promise.then(&[capture_value])?;
    resolver.resolve(None, &[JSValue::string(&ctx, "ready")])?;
    ctx.run_microtasks();

    let result = ctx
        .evaluate_script("globalThis.promiseResult", None)?
        .as_string()?
        .to_string();
    assert_eq!(result, "ready");

    let handler =
        JSFunction::callback(&ctx, Some("recordUnhandled"), Some(record_unhandled));
    let handler_object: JSObject = handler.into();
    let _handler_guard = ctx.set_unhandled_rejection_handler(&handler_object)?;

    let (_rejected, rejecter) = Promise::new_pending(&ctx)?;
    rejecter.reject(None, &[JSValue::string(&ctx, "boom")])?;
    ctx.run_microtasks();

    let unhandled = ctx
        .evaluate_script("globalThis.unhandledReason", None)?
        .as_string()?
        .to_string();
    assert_eq!(unhandled, "boom");

    println!("promises: result={result} unhandled={unhandled}");
    Ok(())
}
