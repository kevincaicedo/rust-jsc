use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{
    callback, CallbackContext, JSClass, JSClassAccessor, JSContext, JSError, JSObject,
    JSResult, JSValue, ThisObject,
};

#[derive(Debug)]
struct BenchCounter {
    value: i32,
}

struct CountAccessor;

impl JSClassAccessor for CountAccessor {
    type Value = i32;

    fn get(ctx: JSContext, object: JSObject) -> JSResult<Self::Value> {
        let Some(state) = object.get_private_data::<BenchCounter>() else {
            return Err(JSError::new_typ(
                &ctx,
                "BenchCounter private data is missing",
            )?);
        };
        Ok(state.value)
    }

    fn set(ctx: JSContext, object: JSObject, value: Self::Value) -> JSResult<()> {
        let Some(mut state) = object.get_private_data_mut::<BenchCounter>() else {
            return Err(JSError::new_typ(
                &ctx,
                "BenchCounter private data is borrowed",
            )?);
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
    let Some(mut state) = this.get_private_data_mut::<BenchCounter>() else {
        return Err(JSError::new_typ(
            &ctx,
            "BenchCounter private data is borrowed",
        )?);
    };
    state.value += amount.unwrap_or(1);
    Ok(state.value)
}

fn bench_class_build(c: &mut Criterion) {
    c.bench_function("class_build_counter", |b| {
        b.iter(|| {
            let class = JSClass::try_builder("BenchCounter")
                .and_then(|builder| builder.method("increment", Some(increment)))
                .and_then(|builder| builder.typed_accessor::<CountAccessor>("count"))
                .and_then(|builder| builder.build::<BenchCounter>())
                .unwrap();
            black_box(class);
        });
    });
}

fn bench_class_object_create(c: &mut Criterion) {
    let ctx = JSContext::new();
    let class = JSClass::try_builder("BenchCounter")
        .and_then(|builder| builder.method("increment", Some(increment)))
        .and_then(|builder| builder.typed_accessor::<CountAccessor>("count"))
        .and_then(|builder| builder.build::<BenchCounter>())
        .unwrap();

    c.bench_function("class_object_create", |b| {
        b.iter(|| {
            let object = class.object(&ctx, Some(BenchCounter { value: 1 }));
            black_box(object);
        });
    });
}

fn bench_class_method_call(c: &mut Criterion) {
    let ctx = JSContext::new();
    let class = JSClass::try_builder("BenchCounter")
        .and_then(|builder| builder.method("increment", Some(increment)))
        .and_then(|builder| builder.typed_accessor::<CountAccessor>("count"))
        .and_then(|builder| builder.build::<BenchCounter>())
        .unwrap();
    let object = class.object(&ctx, Some(BenchCounter { value: 1 }));
    let args = [JSValue::number(&ctx, 1.0)];

    c.bench_function("class_method_call_increment", |b| {
        b.iter(|| {
            let result = object.call_method("increment", &args).unwrap();
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_class_build,
    bench_class_object_create,
    bench_class_method_call,
);
