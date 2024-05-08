
use std::{ffi::{CString, NulError}, thread::sleep};

pub use rust_jsc::bindings::{
    JSContextGroupCreate, JSGlobalContextCreateInGroup, JSLoadAndEvaluateModule,
    JSStringCreateWithUTF8CString, JSStringRef, JSEvaluateScript, JSValueRef,
    JSValueToObject, JSValueToStringCopy, JSStringGetMaximumUTF8CStringSize,
    JSStringGetUTF8CString, JSValueGetType
};

pub fn from_utf8(value: String) -> JSStringRef {
    let value = CString::new(value.as_bytes()).unwrap();
    let inner = unsafe { JSStringCreateWithUTF8CString(value.as_ptr()) };
    inner
}

/// Returns the `JSString` as a Rust `String`
pub fn to_string_utf8(string: JSStringRef) -> String {
    let len = unsafe { JSStringGetMaximumUTF8CStringSize(string) };
    let mut chars = vec![0u8; len as usize];
    let len = unsafe { JSStringGetUTF8CString(string, chars.as_mut_ptr() as _, len) };
    String::from_utf8(chars[0..(len - 1) as usize].to_vec()).unwrap()
}

fn main() {
    // let framework_dir = "/Users/kcaicedo/Documents/Projects/WebKit/WebKitBuild/Release";
    // println!("cargo:rustc-link-search=framework={}", framework_dir);
    // println!("cargo:rustc-link-lib=framework=JavaScriptCore");

    let context_group = unsafe { JSContextGroupCreate() };
    let global_context =
        unsafe { JSGlobalContextCreateInGroup(context_group, std::ptr::null_mut()) };

        // let this_object = std::ptr::null_mut();
        // let source_url = std::ptr::null_mut();
        let mut exception: JSValueRef = std::ptr::null_mut();

        let script = r#"
        import { myFunction } from './script.mjs';

        console.log(myFunction());

        var arr = [];
        // (async () => {
        //     try {
        //         await import('file://script.mjs').then(module => { module.myFunction(); })
        //     } catch (error) {
        //         errorMessage = String(error);
        //         console.log(errorMessage);
        //     }
        //     await 1;
        //     arr.push(3);
        // })();
        arr.push(1);
        arr.push(2);
        arr;
    "#;
    unsafe {
        // JSEvaluateScript(
        //     global_context,
        //     from_utf8(script.to_string()),
        //     this_object,
        //     std::ptr::null_mut(),
        //     1,
        //     &mut exception_1,
        // )
        JSLoadAndEvaluateModule(global_context, from_utf8("../../../scripts/test.js".to_string()), &mut exception)
    };

    let mtype = unsafe {
        JSValueGetType(global_context, exception)
    };
    println!("Type: {:?}", mtype);
    // let mut exception: JSValueRef = std::ptr::null_mut();
    let mut exception2: JSValueRef = std::ptr::null_mut();
    let object_ref = unsafe { JSValueToObject(global_context, exception, &mut exception2) };
    if !exception.is_null() {
        println!("Exception: {:?}", exception);
    }

    let string = unsafe { JSValueToStringCopy(global_context, object_ref, &mut exception) };
    println!("Result: {:?}", to_string_utf8(string));

    // sleep(std::time::Duration::from_secs(1));
}
