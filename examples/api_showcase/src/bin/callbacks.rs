use rust_jsc::{
    callback, CallbackContext, JSContext, JSError, JSFunction, JSObject, JSResult,
    JSValue, Rest, ThisObject,
};

#[callback]
fn add(left: f64, right: f64) -> f64 {
    left + right
}

#[callback]
fn greet(name: Option<String>, labels: Rest<String>) -> JSResult<String> {
    let name = name.unwrap_or_else(|| "world".to_string());
    let suffix = if labels.is_empty() {
        "plain".to_string()
    } else {
        labels.as_slice().join("+")
    };
    Ok(format!("hello {name}:{suffix}"))
}

#[callback]
fn bump(ctx: CallbackContext, this: ThisObject, amount: Option<i32>) -> JSResult<i32> {
    let current = this.get_property("total")?.as_number()? as i32;
    let next = current + amount.unwrap_or(1);
    this.set_property(
        "total",
        &JSValue::number(&ctx, next as f64),
        Default::default(),
    )?;
    Ok(next)
}

fn install_function(
    ctx: &JSContext,
    global: &JSObject,
    name: &str,
    callback: rust_jsc::internal::JSObjectCallAsFunctionCallback,
) -> JSResult<()> {
    let function = JSFunction::callback(ctx, Some(name), callback);
    let value: JSValue = function.into();
    global.set_property(name, &value, Default::default())
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let global = ctx.global_object();

    install_function(&ctx, &global, "add", Some(add))?;
    install_function(&ctx, &global, "greet", Some(greet))?;

    let state = JSObject::new(&ctx);
    state.set_property("total", &JSValue::number(&ctx, 0.0), Default::default())?;
    install_function(&ctx, &state, "bump", Some(bump))?;
    let state_value: JSValue = state.into();
    global.set_property("state", &state_value, Default::default())?;

    let summary = ctx
        .evaluate_script(
            r#"
            const first = add(20, 22);
            const second = greet(undefined, "typed", "rest");
            const third = state.bump(5) + state.bump();
            `${first}:${second}:${third}:${state.total}`;
            "#,
            None,
        )?
        .as_string()?
        .to_string();

    if summary != "42:hello world:typed+rest:11:6" {
        return Err(JSError::new_typ(
            &ctx,
            format!("unexpected callback summary: {summary}"),
        )?);
    }

    println!("callbacks: {summary}");
    Ok(())
}
