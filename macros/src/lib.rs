use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

mod internal;

#[proc_macro_attribute]
pub fn callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) = internal::validate_abi_role(&input, internal::AbiRole::Callback) {
        return TokenStream::from(error.into_compile_error());
    }

    let callback_args = match internal::callback_arguments(&input) {
        Ok(args) => args,
        Err(error) => return TokenStream::from(error.into_compile_error()),
    };

    let (raw_arguments, parse_stmts, func_call) = match callback_args {
        internal::MacroArguments::LegacyRawSlice { call_args } => (
            internal::raw_argument_view(),
            Vec::new(),
            quote!(#fn_name #turbofish(#(#call_args),*)),
        ),
        internal::MacroArguments::Typed {
            parse_stmts,
            call_args,
        } => (
            internal::raw_argument_refs(),
            parse_stmts,
            quote!(#fn_name #turbofish(#(#call_args),*)),
        ),
    };
    let exception_ident = syn::Ident::new("__exception", proc_macro2::Span::call_site());
    let result_mapping = internal::map_js_result_to_value(
        quote!(result),
        &exception_ident,
        quote!(std::ptr::null()),
    );

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __function: rust_jsc::internal::JSObjectRef,
            __this_object: rust_jsc::internal::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> *const rust_jsc::internal::OpaqueJSValue
        #where_clause {
            if __ctx_ref.is_null() {
                return std::ptr::null();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the callback. The wrapper does not retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            if __function.is_null() || __this_object.is_null() {
                if !__exception.is_null() {
                    // SAFETY: JavaScriptCore owns the exception out-pointer
                    // when it is non-null; this wrapper reports invalid raw
                    // callback inputs as a JavaScript TypeError.
                    unsafe {
                        *__exception = rust_jsc::JSError::new_typ_raw(
                            &ctx,
                            "JavaScriptCore callback object was null",
                        );
                    }
                }
                return std::ptr::null();
            }

            // SAFETY: JavaScriptCore provided non-null borrowed object handles
            // that belong to `__ctx_ref` for the callback duration.
            let __function_object =
                unsafe { rust_jsc::JSObject::from_raw_unchecked(__function, __ctx_ref) };
            // SAFETY: JavaScriptCore provided non-null borrowed object handles
            // that belong to `__ctx_ref` for the callback duration.
            let __this_object_value =
                unsafe { rust_jsc::JSObject::from_raw_unchecked(__this_object, __ctx_ref) };
            #raw_arguments

            #(#parse_stmts)*

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #func_call
                })()
            })) {
                Ok(result) => result,
                Err(_) => {
                    if !__exception.is_null() {
                        // SAFETY: JavaScriptCore owns the exception
                        // out-pointer when it is non-null; panics are
                        // converted to a JavaScript TypeError before returning
                        // across the C ABI.
                        unsafe {
                            *__exception = rust_jsc::JSError::new_typ_raw(
                                &ctx,
                                "Rust callback panicked",
                            );
                        }
                    }
                    return std::ptr::null();
                }
            };

            let result = rust_jsc::IntoJSResult::into_js_result(result, &ctx);
            #result_mapping
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::Constructor)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let constructor_args = match internal::constructor_arguments(&input) {
        Ok(args) => args,
        Err(error) => return TokenStream::from(error.into_compile_error()),
    };

    let (raw_arguments, parse_stmts, func_call) = match constructor_args {
        internal::MacroArguments::LegacyRawSlice { call_args } => (
            internal::raw_argument_view(),
            Vec::new(),
            quote!(#fn_name #turbofish(#(#call_args),*)),
        ),
        internal::MacroArguments::Typed {
            parse_stmts,
            call_args,
        } => (
            internal::raw_argument_refs(),
            parse_stmts,
            quote!(#fn_name #turbofish(#(#call_args),*)),
        ),
    };
    let exception_ident = syn::Ident::new("__exception", proc_macro2::Span::call_site());
    let result_mapping = internal::map_js_result_to_value(
        quote!(result),
        &exception_ident,
        quote!(std::ptr::null_mut()),
    );

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __constructor: rust_jsc::internal::JSObjectRef,
            __argument_count: usize,
            __arguments: *const rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSValue
        #where_clause {
            if __ctx_ref.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the constructor callback. The wrapper does not retain
            // or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            if __constructor.is_null() {
                if !__exception.is_null() {
                    // SAFETY: JavaScriptCore owns the exception out-pointer
                    // when it is non-null; this wrapper reports invalid raw
                    // constructor inputs as a JavaScript TypeError.
                    unsafe {
                        *__exception = rust_jsc::JSError::new_typ_raw(
                            &ctx,
                            "JavaScriptCore constructor object was null",
                        );
                    }
                }
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore provided a non-null borrowed constructor
            // object that belongs to `__ctx_ref` for the callback duration.
            let __constructor_object =
                unsafe { rust_jsc::JSObject::from_raw_unchecked(__constructor, __ctx_ref) };
            #raw_arguments

            #(#parse_stmts)*

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #func_call
                })()
            })) {
                Ok(result) => result,
                Err(_) => {
                    if !__exception.is_null() {
                        // SAFETY: JavaScriptCore owns the exception
                        // out-pointer when it is non-null; panics are
                        // converted to a JavaScript TypeError before returning
                        // across the C ABI.
                        unsafe {
                            *__exception = rust_jsc::JSError::new_typ_raw(
                                &ctx,
                                "Rust constructor callback panicked",
                            );
                        }
                    }
                    return std::ptr::null_mut();
                }
            };

            let result = rust_jsc::IntoJSResult::into_js_result(result, &ctx);
            #result_mapping
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) = internal::validate_abi_role(&input, internal::AbiRole::Initialize)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __object: rust_jsc::internal::JSObjectRef,
        )
        #where_clause {
            if __ctx_ref.is_null() || __object.is_null() {
                return;
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the initialize callback. The wrapper does not retain
            // or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null borrowed object that
            // belongs to `__ctx_ref` for the initialize callback duration.
            let object =
                unsafe { rust_jsc::JSObject::from_raw_unchecked(__object, __ctx_ref) };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, object)
                })()
            }));
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) = internal::validate_abi_role(&input, internal::AbiRole::Finalize) {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __object: rust_jsc::internal::JSObjectRef,
        )
        #where_clause {
            if __object.is_null() {
                return;
            }

            // SAFETY: JavaScriptCore passes a borrowed object pointer for the
            // duration of the finalize callback. Reading its private-data slot
            // does not take ownership of the object.
            let data_ptr = unsafe { rust_jsc::internal::JSObjectGetPrivate(__object) };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(data_ptr)
                })()
            }));
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::HasInstance)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let exception_ident = syn::Ident::new("__exception", proc_macro2::Span::call_site());
    let result_mapping =
        internal::map_js_result_to_bool(quote!(result), &exception_ident);

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __constructor: rust_jsc::internal::JSObjectRef,
            __possible_instance: rust_jsc::internal::JSValueRef,
            __exception: *mut rust_jsc::internal::JSValueRef,
        ) -> bool
        #where_clause {
            if __ctx_ref.is_null() || __constructor.is_null() || __possible_instance.is_null() {
                return false;
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the has-instance callback. The wrapper does not retain
            // or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided non-null borrowed handles that
            // belong to `__ctx_ref` for the has-instance callback duration.
            let __constructor_object =
                unsafe { rust_jsc::JSObject::from_raw_unchecked(__constructor, __ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the callback duration.
            let __possible_instance_value =
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__possible_instance, __ctx_ref) };

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __constructor_object, __possible_instance_value)
                })()
            })) {
                Ok(result) => result,
                Err(_) => {
                    if !__exception.is_null() {
                        // SAFETY: JavaScriptCore owns the exception
                        // out-pointer when it is non-null; panics are
                        // converted to a JavaScript TypeError before returning
                        // across the C ABI.
                        unsafe {
                            *__exception = rust_jsc::JSError::new_typ_raw(
                                &ctx,
                                "Rust has-instance callback panicked",
                            );
                        }
                    }
                    return false;
                }
            };

            #result_mapping
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleResolve)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __referrer: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSString
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the module resolver callback. The wrapper does not
            // retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __referrer_value = if __referrer.is_null() {
                rust_jsc::JSValue::undefined(&ctx)
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__referrer, __ctx_ref) }
            };
            let __script_fetcher_value = if __script_fetcher.is_null() {
                rust_jsc::JSValue::undefined(&ctx)
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__script_fetcher, __ctx_ref) }
            };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __key, __referrer_value, __script_fetcher_value)
                })()
            })) {
                Ok(result) => rust_jsc::internal::JSStringRef::from(result),
                Err(_) => std::ptr::null_mut(),
            }
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleEvaluate)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
        ) -> *const rust_jsc::internal::OpaqueJSValue
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the module evaluate callback. The wrapper does not
            // retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __key)
                })()
            })) {
                Ok(result) => result.into(),
                Err(_) => std::ptr::null(),
            }
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleFetch)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __attributes_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSString
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the module fetch callback. The wrapper does not
            // retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __attributes = if __attributes_value.is_null() {
                rust_jsc::JSValue::undefined(&ctx)
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__attributes_value, __ctx_ref) }
            };
            let __script_fetcher_value = if __script_fetcher.is_null() {
                rust_jsc::JSValue::undefined(&ctx)
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__script_fetcher, __ctx_ref) }
            };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __key, __attributes, __script_fetcher_value)
                })()
            })) {
                Ok(result) => rust_jsc::internal::JSStringRef::from(result),
                Err(_) => std::ptr::null_mut(),
            }
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleImportMeta)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSValue
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the import.meta callback. The wrapper does not retain
            // or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __script_fetcher_value = if __script_fetcher.is_null() {
                rust_jsc::JSValue::undefined(&ctx)
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__script_fetcher, __ctx_ref) }
            };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __key, __script_fetcher_value)
                })()
            })) {
                Ok(result) => rust_jsc::internal::JSObjectRef::from(result),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_resolver(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleResolver)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __referrer: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> *mut rust_jsc::internal::OpaqueJSString
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the module resolver callback. The wrapper does not
            // retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __specifier = match __key.as_string() {
                Ok(value) => value.to_string(),
                Err(_) => return std::ptr::null_mut(),
            };
            let __referrer = if __referrer.is_null() {
                None
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                let __referrer_value =
                    unsafe { rust_jsc::JSValue::from_raw_unchecked(__referrer, __ctx_ref) };
                if __referrer_value.is_undefined() || __referrer_value.is_null() {
                    None
                } else {
                    match __referrer_value.as_string() {
                        Ok(value) => Some(value.to_string()),
                        Err(_) => return std::ptr::null_mut(),
                    }
                }
            };

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __specifier, __referrer)
                })()
            })) {
                Ok(result) => result,
                Err(_) => return std::ptr::null_mut(),
            };

            match rust_jsc::IntoModuleResolveResult::into_module_resolve_result(
                result,
                &ctx,
            ) {
                Ok(Some(resolved)) => rust_jsc::internal::JSStringRef::from(resolved),
                Ok(None) | Err(_) => std::ptr::null_mut(),
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_fetcher(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleFetcher)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __attributes_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> rust_jsc::internal::JSModuleSourceRef
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the module fetch-source callback. The wrapper does
            // not retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __specifier = match __key.as_string() {
                Ok(value) => value.to_string(),
                Err(_) => return std::ptr::null_mut(),
            };
            let __import_type = if __attributes_value.is_null() {
                rust_jsc::ModuleImportType::Unknown
            } else {
                // SAFETY: JavaScriptCore provided a value handle that belongs
                // to `__ctx_ref` for the module callback duration.
                let __attributes = unsafe {
                    rust_jsc::JSValue::from_raw_unchecked(__attributes_value, __ctx_ref)
                };
                rust_jsc::ModuleImportType::from_js_value(&__attributes)
            };

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __specifier, __import_type)
                })()
            })) {
                Ok(result) => result,
                Err(_) => return std::ptr::null_mut(),
            };

            match rust_jsc::IntoModuleSourceResult::into_module_source_result(result, &ctx) {
                Ok(Some(source)) => source.into_raw(),
                Ok(None) | Err(_) => std::ptr::null_mut(),
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn module_import_meta_provider(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let visibility = &input.vis;
    let generics = &input.sig.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::ModuleImportMetaProvider)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __key_value: rust_jsc::internal::JSValueRef,
            __script_fetcher: rust_jsc::internal::JSValueRef,
        ) -> rust_jsc::internal::JSObjectRef
        #where_clause {
            if __ctx_ref.is_null() || __key_value.is_null() {
                return std::ptr::null_mut();
            }

            // SAFETY: JavaScriptCore passes a borrowed context pointer for the
            // duration of the import.meta callback. The wrapper does not
            // retain or release it.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null value handle that
            // belongs to `__ctx_ref` for the module callback duration.
            let __key = unsafe { rust_jsc::JSValue::from_raw_unchecked(__key_value, __ctx_ref) };
            let __specifier = match __key.as_string() {
                Ok(value) => value.to_string(),
                Err(_) => return std::ptr::null_mut(),
            };

            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __specifier)
                })()
            })) {
                Ok(result) => result,
                Err(_) => return std::ptr::null_mut(),
            };

            match rust_jsc::IntoImportMetaResult::into_import_meta_result(result, &ctx) {
                Ok(Some(object)) => rust_jsc::internal::JSObjectRef::from(object),
                Ok(None) | Err(_) => std::ptr::null_mut(),
            }
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::UncaughtException)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __filename: rust_jsc::internal::JSStringRef,
            __exception: rust_jsc::internal::JSValueRef,
        ) #where_clause {
            if __ctx_ref.is_null() || __filename.is_null() || __exception.is_null() {
                return;
            }

            // SAFETY: JavaScriptCore passes borrowed raw values for the
            // duration of the uncaught-exception callback. The context wrapper
            // does not retain or release the context.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provides a borrowed string valid for the
            // callback duration; retaining gives Rust an owned wrapper.
            let __filename_value = unsafe { rust_jsc::JSString::retain_from_ref(__filename) };
            // SAFETY: JavaScriptCore provided a non-null exception value that
            // belongs to `__ctx_ref` for the callback duration.
            let __exception_value =
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__exception, __ctx_ref) };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __filename_value, __exception_value)
                })()
            }));
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::UncaughtExceptionEventLoop)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            __ctx_ref: rust_jsc::internal::JSContextRef,
            __exception: rust_jsc::internal::JSValueRef,
        ) #where_clause {
            if __ctx_ref.is_null() || __exception.is_null() {
                return;
            }

            // SAFETY: JavaScriptCore passes borrowed raw values for the
            // duration of the event-loop exception callback. The context
            // wrapper does not retain or release the context.
            let ctx = unsafe { rust_jsc::JSContext::borrowed(__ctx_ref) };
            // SAFETY: JavaScriptCore provided a non-null exception value that
            // belongs to `__ctx_ref` for the callback duration.
            let __exception_value =
                unsafe { rust_jsc::JSValue::from_raw_unchecked(__exception, __ctx_ref) };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(ctx, __exception_value)
                })()
            }));
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) =
        internal::validate_abi_role(&input, internal::AbiRole::InspectorCallback)
    {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            message: *const std::os::raw::c_char,
            message_len: usize,
        ) #where_clause {
            if message.is_null() {
                return;
            }

            // SAFETY: JavaScriptCore passes `message_len` borrowed bytes for
            // the duration of the inspector callback. The wrapper does not
            // retain the pointer beyond this call.
            let message_bytes = unsafe {
                std::slice::from_raw_parts(message.cast::<u8>(), message_len)
            };
            let __message_lossy;
            let __message_str = match std::str::from_utf8(message_bytes) {
                Ok(message) => message,
                Err(_) => {
                    __message_lossy = std::string::String::from_utf8_lossy(message_bytes);
                    __message_lossy.as_ref()
                }
            };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(__message_str)
                })()
            }));
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
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();

    if let Err(error) = internal::validate_abi_role(
        &input,
        internal::AbiRole::InspectorPauseEventCallback,
    ) {
        return TokenStream::from(error.into_compile_error());
    }

    let expanded = quote! {
        #visibility unsafe extern "C" fn #fn_name #impl_generics (
            ctx: rust_jsc::internal::JSContextRef,
            event: rust_jsc::internal::InspectorPauseEvent
        ) #where_clause {
            if ctx.is_null() {
                return;
            }

            // Map the C enum to the Rust enum.
            let event = match event {
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventPaused => rust_jsc::context::InspectorPauseEvent::Paused,
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventResumed => rust_jsc::context::InspectorPauseEvent::Resumed,
                rust_jsc::internal::InspectorPauseEvent_InspectorPauseEventTick => rust_jsc::context::InspectorPauseEvent::Tick,
                _ => return,
            };

            // Convert raw context ref to safe wrapper without taking ownership.
            let js_ctx = unsafe { rust_jsc::JSContext::borrowed(ctx) };

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (|| {
                    #input
                    #fn_name #turbofish(js_ctx, event)
                })()
            }));
        }
    };

    TokenStream::from(expanded)
}
