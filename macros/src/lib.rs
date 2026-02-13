use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, PatType, Type, TypePath};

#[proc_macro_attribute]
pub fn callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    // Collect typed params (excluding the first three)
    // e.g. ctx, func, this -> skip them
    let params: Vec<_> = input.sig.inputs.iter().skip(3).collect();

    // Check if using raw arguments slice
    if params.len() == 1 {
        if let FnArg::Typed(PatType { ty, .. }) = &params[0] {
            if let Type::Reference(_) = &**ty {
                // Handle old-style with raw arguments slice
                return generate_legacy_callback(&input, fn_name, visibility, generics);
            }
        }
    }

    // Generate argument parsing code
    let mut parse_stmts = Vec::new();
    for (i, param) in params.iter().enumerate() {
        if let FnArg::Typed(PatType { pat, ty, .. }) = param {
            let idx = syn::Index::from(i);
            let var_ident = format_ident!("arg_{}", i);
            let param_name = quote!(#pat).to_string();

            parse_stmts.push(match &**ty {
                Type::Path(TypePath { path, .. }) => {
                    let is_optional = path
                        .segments
                        .last()
                        .map(|s| s.ident == "Option")
                        .unwrap_or(false);

                    if is_optional {
                        generate_optional_param_parsing(idx, var_ident)
                    } else {
                        generate_required_param_parsing(
                            idx,
                            var_ident,
                            fn_name.to_string().as_str(),
                            param_name.as_str(),
                        )
                    }
                }
                _ => quote! {
                    panic!("[callback] Unsupported parameter type for {}", #param_name);
                },
            });
        }
    }

    let call_args: Vec<_> = params
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let var_ident = format_ident!("arg_{}", i);
            quote!(#var_ident)
        })
        .collect();

    let func_call = quote! {
        #fn_name ::<#generic_params>(ctx, function, this_object, #(#call_args),*)
    };

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

            #(#parse_stmts)*

            let result = (|| {
                #input
                #func_call
            })();

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

fn generate_optional_param_parsing(
    idx: syn::Index,
    var_ident: syn::Ident,
) -> proc_macro2::TokenStream {
    quote! {
        let #var_ident = match arguments.get(#idx).map(|value| value.try_into()) {
            Some(Ok(value)) => Some(value),
            Some(Err(err)) => {
                *__exception = rust_jsc::internal::JSValueRef::from(err) as *mut _;
                return std::ptr::null_mut();
            },
            None => None,
        };
    }
}

fn generate_required_param_parsing(
    idx: syn::Index,
    var_ident: syn::Ident,
    fn_name: &str,
    param_name: &str,
) -> proc_macro2::TokenStream {
    quote! {
        let #var_ident = match arguments.get(#idx).map(|value| value.try_into()) {
            Some(Ok(value)) => value,
            Some(Err(err)) => {
                *__exception = rust_jsc::internal::JSValueRef::from(err) as *mut _;
                return std::ptr::null_mut();
            },
            None => {
                *__exception = rust_jsc::JSError::new_typ_raw(&ctx, format!("[{}] Missing argument {}", #fn_name, #param_name)) as *mut _;
                return std::ptr::null_mut();
            },
        };
    }
}

fn generate_legacy_callback(
    input: &ItemFn,
    fn_name: &syn::Ident,
    visibility: &syn::Visibility,
    generics: &syn::Generics,
) -> TokenStream {
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
            ) -> rust_jsc::JSStringProctected = {
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
            ) -> rust_jsc::JSStringProctected = {
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

#[proc_macro_attribute]
pub fn uncaught_exception(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __filename: rust_jsc::internal::JSStringRef,
            __exception: rust_jsc::internal::JSValueRef,
        ) #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let filename = rust_jsc::JSString::from(__filename);
            let exception = rust_jsc::JSValue::new(__exception, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSString,
                rust_jsc::JSValue,
            ) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(ctx, filename, exception);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn uncaught_exception_event_loop(
    _attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __exception: rust_jsc::internal::JSValueRef,
        ) #where_clause {
            let ctx = rust_jsc::JSContext::from(__ctx_ref);
            let exception = rust_jsc::JSValue::new(__exception, __ctx_ref);

            let func: fn(
                rust_jsc::JSContext,
                rust_jsc::JSValue,
            ) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(ctx, exception);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn inspector_callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            message: *const std::os::raw::c_char
        ) #where_clause {
            let message_str = std::ffi::CStr::from_ptr(message).to_str().expect("[Inspector] Invalid UTF-8");

            let func: fn(&str) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(message_str);
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn inspector_pause_event_callback(
    _attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let generic_params = &generics.params;
    let where_clause = &generics.where_clause;

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name <#generic_params> (
            ctx: rust_jsc::internal::JSContextRef,
            event: rust_jsc::internal::InspectorPauseEvent
        ) #where_clause {
            // Map the C enum to the Rust enum.
            let event = match event {
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventPaused => rust_jsc::context::InspectorPauseEvent::Paused,
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventResumed => rust_jsc::context::InspectorPauseEvent::Resumed,
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventTick => rust_jsc::context::InspectorPauseEvent::Tick,
                _ => return,
            };

            // Convert raw context ref to safe wrapper without taking ownership.
            let js_ctx = rust_jsc::JSContext::from(ctx);

            let func: fn(rust_jsc::JSContext, rust_jsc::context::InspectorPauseEvent) = {
                #input

                #fn_name ::<#generic_params>
            };

            func(js_ctx, event);
        }
    };

    TokenStream::from(expanded)
}
