use rust_jsc::{
    callback, CallbackContext, JSClass, JSClassAccessor, JSContext, JSError, JSObject,
    JSResult, JSValue, ThisObject,
};

#[derive(Debug)]
struct CounterState {
    value: i32,
}

struct CountAccessor;

impl JSClassAccessor for CountAccessor {
    type Value = i32;

    fn get(ctx: JSContext, object: JSObject) -> JSResult<Self::Value> {
        let Some(state) = object.get_private_data::<CounterState>() else {
            return Err(JSError::new_typ(&ctx, "Counter private data is missing")?);
        };
        Ok(state.value)
    }

    fn set(ctx: JSContext, object: JSObject, value: Self::Value) -> JSResult<()> {
        let Some(mut state) = object.get_private_data_mut::<CounterState>() else {
            return Err(JSError::new_typ(&ctx, "Counter private data is borrowed")?);
        };
        state.value = value;
        Ok(())
    }
}

#[callback]
fn increment(
    ctx: CallbackContext,
    this: ThisObject,
    amount: Option<i32>,
) -> JSResult<i32> {
    let Some(mut state) = this.get_private_data_mut::<CounterState>() else {
        return Err(JSError::new_typ(&ctx, "Counter private data is borrowed")?);
    };
    state.value += amount.unwrap_or(1);
    Ok(state.value)
}

#[callback]
fn version() -> &'static str {
    "1.0"
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let class = JSClass::try_builder("Counter")
        .and_then(|builder| builder.method("increment", Some(increment)))
        .and_then(|builder| builder.constructor_method("version", Some(version)))
        .and_then(|builder| builder.typed_accessor::<CountAccessor>("count"))
        .and_then(|builder| builder.build::<CounterState>())
        .expect("static Counter class definition is valid");

    let counter = class.object(&ctx, Some(CounterState { value: 2 }));
    let counter_value: JSValue = counter.into();
    ctx.global_object()
        .set_property("counter", &counter_value, Default::default())?;

    let constructor = class.constructor_object(&ctx)?;
    let constructor_value: JSValue = constructor.into();
    ctx.global_object().set_property(
        "Counter",
        &constructor_value,
        Default::default(),
    )?;

    let summary = ctx
        .evaluate_script(
            r#"
            counter.increment(4);
            counter.count = counter.count + 3;
            `${Counter.version()}:${counter.count}`;
            "#,
            None,
        )?
        .as_string()?
        .to_string();

    assert_eq!(summary, "1.0:9");
    println!("classes: {summary}");
    Ok(())
}
