use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc_sys::JSContextRef,
            __function: rust_jsc_sys::JSObjectRef,
            __this_object: rust_jsc_sys::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc_sys::JSValueRef,
            __exception: *mut rust_jsc_sys::JSValueRef,
        ) -> *const rust_jsc_sys::OpaqueJSValue
        #where_clause {
            let ctx = crate::JSContext::from(__ctx_ref);
            let function = crate::JSObject::from_ref(__function, __ctx_ref);
            let this_object = crate::JSObject::from_ref(__this_object, __ctx_ref);
            let arguments = if __arguments.is_null() || __argument_count == 0 {
                vec![]
            } else {
                unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
                    .iter()
                    .map(|__inner_value| JSValue::new(*__inner_value, __ctx_ref))
                    .collect::<Vec<_>>()
            };

            let func: fn(
                JSContext,
                JSObject,
                JSObject,
                &[JSValue],
            ) -> crate::JSResult<JSValue> = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, function, this_object, arguments.as_slice());

            match result {
                Ok(value) => {
                    *__exception = std::ptr::null_mut();
                    value.into()
                }
                Err(exception) => {
                    *__exception = rust_jsc_sys::JSValueRef::from(exception) as *mut _;
                    std::ptr::null_mut()
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn constructor(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc_sys::JSContextRef,
            __constructor: rust_jsc_sys::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc_sys::JSValueRef,
            __exception: *mut rust_jsc_sys::JSValueRef,
        ) -> *mut rust_jsc_sys::OpaqueJSValue
        #where_clause {
            let ctx = crate::JSContext::from(__ctx_ref);
            let constructor = crate::JSObject::from_ref(__constructor, __ctx_ref);
            let arguments = if __arguments.is_null() || __argument_count == 0 {
                vec![]
            } else {
                unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
                    .iter()
                    .map(|__inner_value| JSValue::new(*__inner_value, __ctx_ref))
                    .collect::<Vec<_>>()
            };

            let func: fn(
                JSContext,
                JSObject,
                &[JSValue],
            ) -> crate::JSResult<JSValue> = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, constructor, arguments.as_slice());

            match result {
                Ok(value) => {
                    *__exception = std::ptr::null_mut();
                    value.into()
                }
                Err(exception) => {
                    *__exception = rust_jsc_sys::JSValueRef::from(exception) as *mut _;
                    std::ptr::null_mut()
                }
            }
        }
    };

    TokenStream::from(expanded)
}
