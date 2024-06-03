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
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __function: rust_jsc::internal::JSObjectRef,
            __this_object: rust_jsc::internal::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> *const rust_jsc::internal::OpaqueJSValue
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let function = rust_jsc::JSObject::from_ref(__function, __ctx_ref);
            let this_object = rust_jsc::JSObject::from_ref(__this_object, __ctx_ref);
            let arguments = if __arguments.is_null() || __argument_count == 0 {
                vec![]
            } else {
                unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
                    .iter()
                    .map(|__inner_value| rust_jsc::JSValue::new(*__inner_value, __ctx_ref))
                    .collect::<Vec<_>>()
            };

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSObject,
                rust_jsc::JSObject,
                &[rust_jsc::JSValue],
            ) -> rust_jsc::JSResult<rust_jsc::JSValue> = {
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
                    *__exception = rust_jsc::internal::JSValueRef::from(exception) as *mut _;
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
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __constructor: rust_jsc::internal::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSValue
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let constructor = rust_jsc::JSObject::from_ref(__constructor, __ctx_ref);
            let arguments = if __arguments.is_null() || __argument_count == 0 {
                vec![]
            } else {
                unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
                    .iter()
                    .map(|__inner_value| rust_jsc::JSValue::new(*__inner_value, __ctx_ref))
                    .collect::<Vec<_>>()
            };

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSObject,
                &[rust_jsc::JSValue],
            ) -> rust_jsc::JSResult<rust_jsc::JSValue> = {
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
                    *__exception = rust_jsc::internal::JSValueRef::from(exception) as *mut _;
                    std::ptr::null_mut()
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn initialize(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __object: rust_jsc::internal::JSObjectRef,
        )
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let object = rust_jsc::JSObject::from_ref(__object, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSObject,
            ) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(ctx, object);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn finalize(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __object: rust_jsc::internal::JSObjectRef,
        )
        #where_clause {
            let data_ptr = rust_jsc::internal::JSObjectGetPrivate(__object);

            let func: fn(
                rust_jsc::PrivateData
            ) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(data_ptr);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn has_instance(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __constructor: rust_jsc::internal::JSObjectRef,
            __possible_instance: rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> bool
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let constructor = rust_jsc::JSObject::from_ref(__constructor, __ctx_ref);
            let possible_instance = rust_jsc::JSValue::new(__possible_instance, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSObject,
                rust_jsc::JSValue,
            ) -> rust_jsc::JSResult<bool> = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, constructor, possible_instance);

            match result {
                Ok(value) => {
                    *__exception = std::ptr::null_mut();
                    value
                }
                Err(exception) => {
                    *__exception = rust_jsc::internal::JSValueRef::from(exception) as *mut _;
                    false
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_resolve(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __referrer: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSString
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let key_value = rust_jsc::JSValue::new(__key_value, __ctx_ref);
            let referrer = rust_jsc::JSValue::new(__referrer, __ctx_ref);
            let script_fetcher = rust_jsc::JSValue::new(__script_fetcher, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSValue,
                rust_jsc::JSValue,
                rust_jsc::JSValue,
            ) -> rust_jsc::JSStringRetain = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, key_value, referrer, script_fetcher);
            rust_jsc::internal::JSStringRef::from(result)
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_evaluate(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
        ) -> *const rust_jsc::internal::OpaqueJSValue
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let key_value = rust_jsc::JSValue::new(__key_value, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSValue,
            ) -> rust_jsc::JSValue = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, key_value);
            result.into()
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_fetch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __attributes_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSString
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let key_value = rust_jsc::JSValue::new(__key_value, __ctx_ref);
            let attributes_value = rust_jsc::JSValue::new(__attributes_value, __ctx_ref);
            let script_fetcher = rust_jsc::JSValue::new(__script_fetcher, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSValue,
                rust_jsc::JSValue,
                rust_jsc::JSValue,
            ) -> rust_jsc::JSStringRetain = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, key_value, attributes_value, script_fetcher);
            rust_jsc::internal::JSStringRef::from(result)
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_import_meta(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSValue
        #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let key_value = rust_jsc::JSValue::new(__key_value, __ctx_ref);
            let script_fetcher = rust_jsc::JSValue::new(__script_fetcher, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSValue,
                rust_jsc::JSValue,
            ) -> rust_jsc::JSObject = {
                #input

                #fn_name ::<#generic_params>
            };

            let result = func(ctx, key_value, script_fetcher);
            rust_jsc::internal::JSObjectRef::from(result)
        }
    };

    TokenStream::from(expanded)
}