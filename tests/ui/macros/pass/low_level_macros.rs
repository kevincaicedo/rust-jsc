use rust_jsc::{
    finalize, has_instance, initialize, inspector_callback,
    inspector_pause_event_callback, module_evaluate, module_fetch, module_import_meta,
    module_resolve, uncaught_exception, uncaught_exception_event_loop,
    InspectorPauseEvent, JSContext, JSObject, JSResult, JSString, JSStringProtected,
    JSValue, PrivateData,
};

#[initialize]
fn initialize(_ctx: JSContext, _object: JSObject) {}

#[finalize]
fn finalize(_data: PrivateData) {}

#[has_instance]
fn has_instance(
    _ctx: JSContext,
    _constructor: JSObject,
    possible_instance: JSValue,
) -> JSResult<bool> {
    Ok(possible_instance.is_object())
}

#[module_resolve]
fn resolve(
    _ctx: JSContext,
    key: JSValue,
    _referrer: JSValue,
    _fetcher: JSValue,
) -> JSStringProtected {
    JSStringProtected::from(key.as_string().unwrap().to_string())
}

#[module_fetch]
fn fetch(
    _ctx: JSContext,
    _key: JSValue,
    _attributes: JSValue,
    _fetcher: JSValue,
) -> JSStringProtected {
    JSStringProtected::from("export default 1;")
}

#[module_import_meta]
fn import_meta(ctx: JSContext, key: JSValue, _fetcher: JSValue) -> JSObject {
    let object = JSObject::new(&ctx);
    object.set_property("url", &key, Default::default()).unwrap();
    object
}

#[module_evaluate]
fn evaluate(ctx: JSContext, _key: JSValue) -> JSValue {
    JSValue::undefined(&ctx)
}

#[uncaught_exception]
fn uncaught(_ctx: JSContext, _filename: JSString, _exception: JSValue) {}

#[uncaught_exception_event_loop]
fn event_loop_exception(_ctx: JSContext, _exception: JSValue) {}

#[inspector_callback]
fn frontend_message(_message: &str) {}

#[inspector_pause_event_callback]
fn pause_event(_ctx: JSContext, _event: InspectorPauseEvent) {}

fn main() {
    let _ = initialize;
    let _ = finalize;
    let _ = has_instance;
    let _ = resolve;
    let _ = fetch;
    let _ = import_meta;
    let _ = evaluate;
    let _ = uncaught;
    let _ = event_loop_exception;
    let _ = frontend_message;
    let _ = pause_event;
}
