// use rust_jsc::internal::{
//     JSContextRef, JSModuleLoaderCallbacks, JSObjectMake, JSObjectRef, JSStringRef,
//     JSValueMakeUndefined, JSValueRef, PropertyDescriptor
// };

use rust_jsc::{
    callback, module_evaluate, module_fetch, module_import_meta, module_resolve,
    JSContext, JSFunction, JSObject, JSResult, JSString, JSStringRetain, JSValue, JSPromise,
    PropertyDescriptorBuilder, JSModuleLoader, PropertyDescriptor,
};

#[callback]
fn log_info(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let message = arguments.get(0).unwrap().as_string().unwrap();
    println!("INFO: {}", message);

    Ok(JSValue::undefined(&ctx))
}

#[callback]
fn set_timeout(
ctx: JSContext,
_function: JSObject,
_this: JSObject,
arguments: &[JSValue],
) -> JSResult<JSValue> {
    println!("Set Timeout");
    let callback = arguments.get(0).unwrap().as_object().unwrap();
    let timeout = arguments.get(1).unwrap().as_number().unwrap();
    // wait for timeout and then call the callback and return the result
    // 1. sleep for timeout
    std::thread::sleep(std::time::Duration::from_millis(timeout as u64));
    // 2. call the callback
    // let result = callback.call(None, &[]);
    // 3. return the result
    Ok(JSValue::undefined(&ctx))
}

#[module_resolve]
fn moduleLoaderResolve(
    ctx: JSContext,
    key: JSValue,
    referrer: JSValue,
    script_fetcher: JSValue,
) -> JSStringRetain {
    let key_value = key.as_string().unwrap();
    // let referrer_value = referrer.as_string().unwrap();
    // let script = script_fetcher.as_string().unwrap();

    // println!("ModuleLoaderResolve, Key: {:?}", key_value);

    JSStringRetain::from("@rust-jsc")
}

#[module_evaluate]
fn moduleLoaderEvaluate(
    ctx: JSContext,
    key: JSValue,
) -> JSValue {
    // let key = key.as_string().unwrap();

    println!("ModuleLoaderEvaluate, Key: {:?}", key.as_string().unwrap());

    let object = JSObject::new(&ctx);
    let keydata = JSValue::string(&ctx, "name");
    let value = JSValue::string(&ctx, "John Doe");
    let result = object.set(&keydata, &value, PropertyDescriptor::default());
    match result {
        Ok(_) => {
            // println!("Set Property");
        }
        Err(error) => {
            println!("Error M: {:?}", error.message().unwrap());
        }
    }

    let default = JSObject::new(&ctx);
    default.set_property("default", &object.into(), PropertyDescriptor::default()).unwrap();
    default.set_property("name", &value, PropertyDescriptor::default()).unwrap();

    default.into()
}

#[module_fetch]
fn moduleLoaderFetch(
    ctx: JSContext,
    key: JSValue,
    attributes_value: JSValue,
    script_fetcher: JSValue,
) -> JSStringRetain {
    let key_value = key.as_string().unwrap();
    let script = script_fetcher.as_string().unwrap();
    let attributes = attributes_value.as_string().unwrap();

    println!("ModuleLoaderFetch, Key: {:?}", key_value);

    JSStringRetain::from("let name = 'Kevin'; export default name;")
}

#[module_import_meta]
fn moduleLoaderCreateImportMetaProperties(
    ctx: JSContext,
    key: JSValue,
    script_fetcher: JSValue,
) -> JSObject {
    let script = script_fetcher.as_string().unwrap();
    // let key_value = key.as_string().unwrap();

    // println!("ImportMeta, Key: {:?}", key_value);

    let object = JSObject::new(&ctx);
    object.set_property("url", &key, Default::default()).unwrap();
    object
}

fn main() {
    let ctx = JSContext::new();
    let global_object = ctx.global_object();

    let object = JSObject::new(&ctx);
    let attributes = PropertyDescriptorBuilder::new()
        .writable(true)
        .configurable(true)
        .enumerable(true)
        .build();
    let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
    object
        .set_property("log", &function.into(), attributes)
        .unwrap();
    let timeout_function = JSFunction::callback(&ctx, Some("setTimeout"), Some(set_timeout));
    object
        .set_property("setTimeout", &timeout_function.into(), attributes)
        .unwrap();

    global_object
        .set_property("console", &object.into(), attributes)
        .unwrap();
    
    ctx.set_inspectable(true);

    let callbacks = JSModuleLoader {
        disableBuiltinFileSystemLoader: false,
        moduleLoaderResolve: Some(moduleLoaderResolve),
        moduleLoaderEvaluate: Some(moduleLoaderEvaluate),
        moduleLoaderFetch: Some(moduleLoaderFetch),
        moduleLoaderCreateImportMetaProperties: Some(
            moduleLoaderCreateImportMetaProperties,
        ),
    };

    ctx.set_module_loader(callbacks);
    
    let keys = &[
        JSStringRetain::from("@rust-jsc"),
    ];
    ctx.set_virtual_module_keys(keys);
    
    // let result = ctx.evaluate_script("console.log('Hello, World!')", None);
    // let result = ctx.evaluate_module("../scripts/test.js");
    let result = ctx.evaluate_module("../scripts/jsc-test.mjs");
    ctx.check_syntax("console.log('Kevin')", 0).unwrap();
    println!("Result: M");
    // let result = ctx.load_module("../scripts/test.js");
    // assert!(result.is_ok());
    // read module from file system
    // let module_source = std::fs::read_to_string("../scripts/output/jsc.js").unwrap();
    // let result = ctx.evaluate_module_from_source(&module_source, "../scripts/output/jsc.js", None);
    // println!("Hello, World!");
    // let result = ctx.link_and_evaluate_module("test.js");
    // println!("Result: {:?}", result.is_undefined());
    match result {
        Ok(value) => {
            println!("Result: {:?}", ctx.check_syntax("console.log('Kevin')", 0).unwrap());
        }
        Err(error) => {
            eprintln!("Error M: {:?}, {:?}", error.message().unwrap(), ctx.check_syntax("console.log('Kevin')", 0).unwrap());
        }
    }
    // assert!(result.is_ok());
}
