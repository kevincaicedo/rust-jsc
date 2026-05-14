use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    AngleBracketedGenericArguments, FnArg, GenericArgument, ItemFn, PatType,
    PathArguments, ReturnType, Type, TypePath,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AbiRole {
    Callback,
    Constructor,
    Initialize,
    Finalize,
    HasInstance,
    ModuleResolve,
    ModuleEvaluate,
    ModuleFetch,
    ModuleImportMeta,
    ModuleResolver,
    ModuleFetcher,
    ModuleImportMetaProvider,
    UncaughtException,
    UncaughtExceptionEventLoop,
    InspectorCallback,
    InspectorPauseEventCallback,
}

impl AbiRole {
    fn name(self) -> &'static str {
        match self {
            Self::Callback => "callback",
            Self::Constructor => "constructor",
            Self::Initialize => "initialize",
            Self::Finalize => "finalize",
            Self::HasInstance => "has_instance",
            Self::ModuleResolve => "module_resolve",
            Self::ModuleEvaluate => "module_evaluate",
            Self::ModuleFetch => "module_fetch",
            Self::ModuleImportMeta => "module_import_meta",
            Self::ModuleResolver => "module_resolver",
            Self::ModuleFetcher => "module_fetcher",
            Self::ModuleImportMetaProvider => "module_import_meta_provider",
            Self::UncaughtException => "uncaught_exception",
            Self::UncaughtExceptionEventLoop => "uncaught_exception_event_loop",
            Self::InspectorCallback => "inspector_callback",
            Self::InspectorPauseEventCallback => "inspector_pause_event_callback",
        }
    }
}

pub(crate) enum MacroArguments {
    LegacyRawSlice {
        call_args: Vec<TokenStream2>,
    },
    Typed {
        parse_stmts: Vec<TokenStream2>,
        call_args: Vec<TokenStream2>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InjectedArgument {
    CallbackContext,
    CallbackFunction,
    ThisObject,
    ConstructorObject,
}

pub(crate) fn validate_abi_role(input: &ItemFn, role: AbiRole) -> syn::Result<()> {
    if input.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            input.sig.asyncness,
            format!("#[{}] does not support async functions", role.name()),
        ));
    }

    if input.sig.constness.is_some() {
        return Err(syn::Error::new_spanned(
            input.sig.constness,
            format!("#[{}] does not support const functions", role.name()),
        ));
    }

    if input.sig.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            input.sig.unsafety,
            format!("#[{}] wraps a safe Rust function in an unsafe extern ABI; the user function must be safe", role.name()),
        ));
    }

    let params = typed_params(input)?;
    match role {
        AbiRole::Callback => {
            let _ = callback_signature(input)?;
        }
        AbiRole::Constructor => {
            let _ = constructor_signature(input)?;
        }
        AbiRole::Initialize => {
            require_param_count(
                input,
                &params,
                2,
                "#[initialize] expects exactly (JSContext, JSObject)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[initialize] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSObject",
                "#[initialize] second argument must be JSObject",
            )?;
            require_no_return(input, "#[initialize] must not return a value")?;
        }
        AbiRole::Finalize => {
            require_param_count(
                input,
                &params,
                1,
                "#[finalize] expects exactly (PrivateData)",
            )?;
            require_type(
                params[0],
                "PrivateData",
                "#[finalize] first argument must be PrivateData",
            )?;
            require_no_return(input, "#[finalize] must not return a value")?;
        }
        AbiRole::HasInstance => {
            if params.len() != 3 {
                return Err(signature_error(
                    input,
                    "#[has_instance] expects exactly (JSContext, JSObject, JSValue)",
                ));
            }
            require_type(
                params[0],
                "JSContext",
                "#[has_instance] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSObject",
                "#[has_instance] second argument must be JSObject",
            )?;
            require_type(
                params[2],
                "JSValue",
                "#[has_instance] third argument must be JSValue",
            )?;
            require_js_result(
                input,
                "bool",
                "#[has_instance] must return JSResult<bool>",
            )?;
        }
        AbiRole::ModuleResolve => {
            require_param_count(
                input,
                &params,
                4,
                "#[module_resolve] expects exactly (JSContext, JSValue, JSValue, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_resolve] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSValue",
                "#[module_resolve] second argument must be JSValue",
            )?;
            require_type(
                params[2],
                "JSValue",
                "#[module_resolve] third argument must be JSValue",
            )?;
            require_type(
                params[3],
                "JSValue",
                "#[module_resolve] fourth argument must be JSValue",
            )?;
            require_return_type_any(
                input,
                &["JSStringProtected", "JSStringProctected"],
                "#[module_resolve] must return JSStringProtected",
            )?;
        }
        AbiRole::ModuleEvaluate => {
            require_param_count(
                input,
                &params,
                2,
                "#[module_evaluate] expects exactly (JSContext, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_evaluate] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSValue",
                "#[module_evaluate] second argument must be JSValue",
            )?;
            require_return_type(
                input,
                "JSValue",
                "#[module_evaluate] must return JSValue",
            )?;
        }
        AbiRole::ModuleFetch => {
            require_param_count(
                input,
                &params,
                4,
                "#[module_fetch] expects exactly (JSContext, JSValue, JSValue, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_fetch] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSValue",
                "#[module_fetch] second argument must be JSValue",
            )?;
            require_type(
                params[2],
                "JSValue",
                "#[module_fetch] third argument must be JSValue",
            )?;
            require_type(
                params[3],
                "JSValue",
                "#[module_fetch] fourth argument must be JSValue",
            )?;
            require_return_type_any(
                input,
                &["JSStringProtected", "JSStringProctected"],
                "#[module_fetch] must return JSStringProtected",
            )?;
        }
        AbiRole::ModuleImportMeta => {
            require_param_count(
                input,
                &params,
                3,
                "#[module_import_meta] expects exactly (JSContext, JSValue, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_import_meta] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSValue",
                "#[module_import_meta] second argument must be JSValue",
            )?;
            require_type(
                params[2],
                "JSValue",
                "#[module_import_meta] third argument must be JSValue",
            )?;
            require_return_type(
                input,
                "JSObject",
                "#[module_import_meta] must return JSObject",
            )?;
        }
        AbiRole::ModuleResolver => {
            require_param_count(
                input,
                &params,
                3,
                "#[module_resolver] expects exactly (JSContext, String, Option<String>)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_resolver] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "String",
                "#[module_resolver] second argument must be String",
            )?;
            require_option_type(
                params[2],
                "String",
                "#[module_resolver] third argument must be Option<String>",
            )?;
            require_return_value(
                input,
                "#[module_resolver] must return a resolved key value",
            )?;
        }
        AbiRole::ModuleFetcher => {
            require_param_count(
                input,
                &params,
                3,
                "#[module_fetcher] expects exactly (JSContext, String, ModuleImportType)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_fetcher] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "String",
                "#[module_fetcher] second argument must be String",
            )?;
            require_type(
                params[2],
                "ModuleImportType",
                "#[module_fetcher] third argument must be ModuleImportType",
            )?;
            require_return_value(
                input,
                "#[module_fetcher] must return a module source value",
            )?;
        }
        AbiRole::ModuleImportMetaProvider => {
            require_param_count(
                input,
                &params,
                2,
                "#[module_import_meta_provider] expects exactly (JSContext, String)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[module_import_meta_provider] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "String",
                "#[module_import_meta_provider] second argument must be String",
            )?;
            require_return_value(
                input,
                "#[module_import_meta_provider] must return an import-meta object value",
            )?;
        }
        AbiRole::UncaughtException => {
            require_param_count(
                input,
                &params,
                3,
                "#[uncaught_exception] expects exactly (JSContext, JSString, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[uncaught_exception] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSString",
                "#[uncaught_exception] second argument must be JSString",
            )?;
            require_type(
                params[2],
                "JSValue",
                "#[uncaught_exception] third argument must be JSValue",
            )?;
            require_no_return(input, "#[uncaught_exception] must not return a value")?;
        }
        AbiRole::UncaughtExceptionEventLoop => {
            require_param_count(
                input,
                &params,
                2,
                "#[uncaught_exception_event_loop] expects exactly (JSContext, JSValue)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[uncaught_exception_event_loop] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "JSValue",
                "#[uncaught_exception_event_loop] second argument must be JSValue",
            )?;
            require_no_return(
                input,
                "#[uncaught_exception_event_loop] must not return a value",
            )?;
        }
        AbiRole::InspectorCallback => {
            require_param_count(
                input,
                &params,
                1,
                "#[inspector_callback] expects exactly (&str)",
            )?;
            if !is_str_reference(&params[0].ty) {
                return Err(syn::Error::new_spanned(
                    &params[0].ty,
                    "#[inspector_callback] first argument must be &str",
                ));
            }
            require_no_return(input, "#[inspector_callback] must not return a value")?;
        }
        AbiRole::InspectorPauseEventCallback => {
            require_param_count(
                input,
                &params,
                2,
                "#[inspector_pause_event_callback] expects exactly (JSContext, InspectorPauseEvent)",
            )?;
            require_type(
                params[0],
                "JSContext",
                "#[inspector_pause_event_callback] first argument must be JSContext",
            )?;
            require_type(
                params[1],
                "InspectorPauseEvent",
                "#[inspector_pause_event_callback] second argument must be InspectorPauseEvent",
            )?;
            require_no_return(
                input,
                "#[inspector_pause_event_callback] must not return a value",
            )?;
        }
    }

    Ok(())
}

pub(crate) fn callback_arguments(input: &ItemFn) -> syn::Result<MacroArguments> {
    let (injected_args, user_params) = callback_signature(input)?;

    macro_arguments(
        input,
        injected_args,
        &user_params,
        quote!(std::ptr::null()),
        "#[callback]",
    )
}

pub(crate) fn constructor_arguments(input: &ItemFn) -> syn::Result<MacroArguments> {
    let (injected_args, user_params) = constructor_signature(input)?;

    macro_arguments(
        input,
        injected_args,
        &user_params,
        quote!(std::ptr::null_mut()),
        "#[constructor]",
    )
}

fn callback_signature(input: &ItemFn) -> syn::Result<(Vec<TokenStream2>, Vec<&PatType>)> {
    let params = typed_params(input)?;
    if has_legacy_callback_prefix(&params) {
        return Ok((
            vec![
                quote!(ctx),
                quote!(__function_object),
                quote!(__this_object_value),
            ],
            params[3..].to_vec(),
        ));
    }

    let mut injected_args = Vec::new();
    let mut seen_context = false;
    let mut seen_function = false;
    let mut seen_this = false;
    let mut seen_constructor = false;
    let mut user_start = 0;

    for param in &params {
        let Some(injection) = callback_injection(&param.ty) else {
            break;
        };

        push_unique_injection(
            param,
            injection,
            &mut seen_context,
            &mut seen_function,
            &mut seen_this,
            &mut seen_constructor,
        )?;
        injected_args.push(injection.expr());
        user_start += 1;
    }

    reject_late_callback_injections(&params[user_start..])?;
    Ok((injected_args, params[user_start..].to_vec()))
}

fn constructor_signature(
    input: &ItemFn,
) -> syn::Result<(Vec<TokenStream2>, Vec<&PatType>)> {
    let params = typed_params(input)?;
    if has_legacy_constructor_prefix(&params) {
        return Ok((
            vec![quote!(ctx), quote!(__constructor_object)],
            params[2..].to_vec(),
        ));
    }

    let mut injected_args = Vec::new();
    let mut seen_context = false;
    let mut seen_function = false;
    let mut seen_this = false;
    let mut seen_constructor = false;
    let mut user_start = 0;

    for param in &params {
        let Some(injection) = constructor_injection(&param.ty) else {
            break;
        };

        push_unique_injection(
            param,
            injection,
            &mut seen_context,
            &mut seen_function,
            &mut seen_this,
            &mut seen_constructor,
        )?;
        injected_args.push(injection.expr());
        user_start += 1;
    }

    reject_late_constructor_injections(&params[user_start..])?;
    Ok((injected_args, params[user_start..].to_vec()))
}

fn has_legacy_callback_prefix(params: &[&PatType]) -> bool {
    params.len() >= 3
        && has_type(params[0], "JSContext")
        && has_type(params[1], "JSObject")
        && has_type(params[2], "JSObject")
}

fn has_legacy_constructor_prefix(params: &[&PatType]) -> bool {
    params.len() >= 2
        && has_type(params[0], "JSContext")
        && has_type(params[1], "JSObject")
}

fn has_type(param: &PatType, expected: &str) -> bool {
    type_path_last_ident(&param.ty).is_some_and(|ident| ident == expected)
}

fn callback_injection(ty: &Type) -> Option<InjectedArgument> {
    match type_path_last_ident(ty)
        .map(|ident| ident.to_string())?
        .as_str()
    {
        "CallbackContext" => Some(InjectedArgument::CallbackContext),
        "CallbackFunction" => Some(InjectedArgument::CallbackFunction),
        "ThisObject" => Some(InjectedArgument::ThisObject),
        _ => None,
    }
}

fn constructor_injection(ty: &Type) -> Option<InjectedArgument> {
    match type_path_last_ident(ty)
        .map(|ident| ident.to_string())?
        .as_str()
    {
        "CallbackContext" => Some(InjectedArgument::CallbackContext),
        "ConstructorObject" => Some(InjectedArgument::ConstructorObject),
        _ => None,
    }
}

fn push_unique_injection(
    param: &PatType,
    injection: InjectedArgument,
    seen_context: &mut bool,
    seen_function: &mut bool,
    seen_this: &mut bool,
    seen_constructor: &mut bool,
) -> syn::Result<()> {
    let (seen, name) = match injection {
        InjectedArgument::CallbackContext => (seen_context, "CallbackContext"),
        InjectedArgument::CallbackFunction => (seen_function, "CallbackFunction"),
        InjectedArgument::ThisObject => (seen_this, "ThisObject"),
        InjectedArgument::ConstructorObject => (seen_constructor, "ConstructorObject"),
    };

    if *seen {
        Err(syn::Error::new_spanned(
            &param.ty,
            format!("duplicate {name} injection parameter"),
        ))
    } else {
        *seen = true;
        Ok(())
    }
}

fn reject_late_callback_injections(params: &[&PatType]) -> syn::Result<()> {
    for param in params {
        if callback_injection(&param.ty).is_some() {
            return Err(syn::Error::new_spanned(
                &param.ty,
                "#[callback] injection parameters must come before JavaScript arguments",
            ));
        }

        if has_type(param, "JSContext") {
            return Err(syn::Error::new_spanned(
                &param.ty,
                "#[callback] ergonomic signatures use CallbackContext for context injection; the legacy prefix is exactly (JSContext, JSObject, JSObject)",
            ));
        }
    }

    Ok(())
}

fn reject_late_constructor_injections(params: &[&PatType]) -> syn::Result<()> {
    for param in params {
        if constructor_injection(&param.ty).is_some() {
            return Err(syn::Error::new_spanned(
                &param.ty,
                "#[constructor] injection parameters must come before JavaScript arguments",
            ));
        }

        if has_type(param, "JSContext") {
            return Err(syn::Error::new_spanned(
                &param.ty,
                "#[constructor] ergonomic signatures use CallbackContext for context injection; the legacy prefix is exactly (JSContext, JSObject)",
            ));
        }
    }

    Ok(())
}

impl InjectedArgument {
    fn expr(self) -> TokenStream2 {
        match self {
            Self::CallbackContext => quote!(rust_jsc::CallbackContext::from(ctx)),
            Self::CallbackFunction => {
                quote!(rust_jsc::CallbackFunction::from(__function_object))
            }
            Self::ThisObject => quote!(rust_jsc::ThisObject::from(__this_object_value)),
            Self::ConstructorObject => {
                quote!(rust_jsc::ConstructorObject::from(__constructor_object))
            }
        }
    }
}

fn macro_arguments(
    input: &ItemFn,
    injected_args: Vec<TokenStream2>,
    user_params: &[&PatType],
    failure_return: TokenStream2,
    macro_name: &str,
) -> syn::Result<MacroArguments> {
    if user_params.len() == 1 && is_js_value_slice_reference(&user_params[0].ty) {
        let mut call_args = injected_args;
        call_args.push(quote!(arguments.as_slice()));
        return Ok(MacroArguments::LegacyRawSlice { call_args });
    }

    let mut parse_stmts = Vec::with_capacity(user_params.len());
    let mut call_args = Vec::with_capacity(injected_args.len() + user_params.len());
    call_args.extend(injected_args);

    for (i, param) in user_params.iter().enumerate() {
        if matches!(&*param.ty, Type::Reference(_)) {
            return Err(syn::Error::new_spanned(
                &param.ty,
                format!(
                    "typed {macro_name} parameters must be owned conversion types; the legacy raw argument form is exactly &[JSValue]"
                ),
            ));
        }

        if !matches!(&*param.ty, Type::Path(_)) {
            return Err(syn::Error::new_spanned(
                &param.ty,
                format!("unsupported {macro_name} parameter type"),
            ));
        }

        let is_rest = is_rest_type(&param.ty);
        if is_rest && i + 1 != user_params.len() {
            return Err(syn::Error::new_spanned(
                &param.ty,
                format!("Rest<T> must be the final typed {macro_name} parameter"),
            ));
        }

        let idx = syn::Index::from(i);
        let var_ident = format_ident!("arg_{}", i);
        let param_name = param.pat.to_token_stream().to_string();
        parse_stmts.push(if is_rest {
            generate_rest_param_parsing(idx, &var_ident, failure_return.clone())
        } else if is_option_type(&param.ty) {
            generate_optional_param_parsing(idx, &var_ident, failure_return.clone())
        } else {
            generate_required_param_parsing(
                idx,
                &var_ident,
                &input.sig.ident.to_string(),
                &param_name,
                failure_return.clone(),
            )
        });
        call_args.push(quote!(#var_ident));
    }

    Ok(MacroArguments::Typed {
        parse_stmts,
        call_args,
    })
}

pub(crate) fn raw_argument_view() -> TokenStream2 {
    quote! {
        let arguments = if __arguments.is_null() || __argument_count == 0 {
            std::vec::Vec::new()
        } else {
            // SAFETY: JavaScriptCore passes `__argument_count` entries when
            // `__arguments` is non-null for callback/constructor ABIs.
            unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
                .iter()
                .map(|__inner_value| {
                    // SAFETY: JavaScriptCore provided each argument value as
                    // belonging to `__ctx_ref` for the callback duration.
                    unsafe {
                        rust_jsc::JSValue::from_raw_unchecked(*__inner_value, __ctx_ref)
                    }
                })
                .collect::<std::vec::Vec<_>>()
        };
    }
}

pub(crate) fn raw_argument_refs() -> TokenStream2 {
    quote! {
        let __raw_arguments: &[rust_jsc::internal::JSValueRef] =
            if __arguments.is_null() || __argument_count == 0 {
                &[]
            } else {
                // SAFETY: JavaScriptCore passes `__argument_count` entries when
                // `__arguments` is non-null for callback/constructor ABIs. The generated
                // wrapper only borrows this view for the callback duration.
                unsafe { std::slice::from_raw_parts(__arguments, __argument_count) }
            };
    }
}

pub(crate) fn map_js_result_to_value(
    result_expr: TokenStream2,
    exception_ident: &syn::Ident,
    null_expr: TokenStream2,
) -> TokenStream2 {
    let clear_exception = clear_exception(exception_ident);
    let write_exception = write_exception(
        exception_ident,
        quote!(rust_jsc::internal::JSValueRef::from(exception) as *mut _),
    );

    quote! {
        match #result_expr {
            Ok(value) => {
                #clear_exception
                value.into()
            }
            Err(exception) => {
                #write_exception
                #null_expr
            }
        }
    }
}

pub(crate) fn map_js_result_to_bool(
    result_expr: TokenStream2,
    exception_ident: &syn::Ident,
) -> TokenStream2 {
    let clear_exception = clear_exception(exception_ident);
    let write_exception = write_exception(
        exception_ident,
        quote!(rust_jsc::internal::JSValueRef::from(exception) as *mut _),
    );

    quote! {
        match #result_expr {
            Ok(value) => {
                #clear_exception
                value
            }
            Err(exception) => {
                #write_exception
                false
            }
        }
    }
}

fn typed_params(input: &ItemFn) -> syn::Result<Vec<&PatType>> {
    let mut params = Vec::with_capacity(input.sig.inputs.len());
    for arg in &input.sig.inputs {
        match arg {
            FnArg::Typed(param) => params.push(param),
            FnArg::Receiver(receiver) => {
                return Err(syn::Error::new_spanned(
                    receiver,
                    "rust-jsc callback macros do not support method receivers",
                ));
            }
        }
    }
    Ok(params)
}

fn require_type(param: &PatType, expected: &str, message: &str) -> syn::Result<()> {
    if type_path_last_ident(&param.ty).is_some_and(|ident| ident == expected) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(&param.ty, message))
    }
}

fn require_option_type(
    param: &PatType,
    expected_inner: &str,
    message: &str,
) -> syn::Result<()> {
    if option_inner_ident(&param.ty).is_some_and(|ident| ident == expected_inner) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(&param.ty, message))
    }
}

fn require_param_count(
    input: &ItemFn,
    params: &[&PatType],
    expected: usize,
    message: &str,
) -> syn::Result<()> {
    if params.len() == expected {
        Ok(())
    } else {
        Err(signature_error(input, message))
    }
}

fn require_return_type(input: &ItemFn, expected: &str, message: &str) -> syn::Result<()> {
    match &input.sig.output {
        ReturnType::Type(_, ty)
            if type_path_last_ident(ty).is_some_and(|ident| ident == expected) =>
        {
            Ok(())
        }
        ReturnType::Type(_, ty) => Err(syn::Error::new_spanned(ty, message)),
        ReturnType::Default => Err(signature_error(input, message)),
    }
}

fn require_return_type_any(
    input: &ItemFn,
    expected: &[&str],
    message: &str,
) -> syn::Result<()> {
    match &input.sig.output {
        ReturnType::Type(_, ty)
            if type_path_last_ident(ty).is_some_and(|ident| {
                expected.iter().any(|expected| ident == expected)
            }) =>
        {
            Ok(())
        }
        ReturnType::Type(_, ty) => Err(syn::Error::new_spanned(ty, message)),
        ReturnType::Default => Err(signature_error(input, message)),
    }
}

fn require_js_result(
    input: &ItemFn,
    expected_inner: &str,
    message: &str,
) -> syn::Result<()> {
    match &input.sig.output {
        ReturnType::Type(_, ty)
            if js_result_inner_ident(ty).is_some_and(|ident| ident == expected_inner) =>
        {
            Ok(())
        }
        ReturnType::Type(_, ty) => Err(syn::Error::new_spanned(ty, message)),
        ReturnType::Default => Err(signature_error(input, message)),
    }
}

fn require_no_return(input: &ItemFn, message: &str) -> syn::Result<()> {
    match &input.sig.output {
        ReturnType::Default => Ok(()),
        ReturnType::Type(_, ty) if is_unit_type(ty) => Ok(()),
        ReturnType::Type(_, ty) => Err(syn::Error::new_spanned(ty, message)),
    }
}

fn require_return_value(input: &ItemFn, message: &str) -> syn::Result<()> {
    match &input.sig.output {
        ReturnType::Default => Err(signature_error(input, message)),
        ReturnType::Type(_, ty) if is_unit_type(ty) => {
            Err(syn::Error::new_spanned(ty, message))
        }
        ReturnType::Type(_, _) => Ok(()),
    }
}

fn signature_error(input: &ItemFn, message: &str) -> syn::Error {
    syn::Error::new(input.sig.ident.span(), message)
}

fn clear_exception(exception_ident: &syn::Ident) -> TokenStream2 {
    quote! {
        if !#exception_ident.is_null() {
            // SAFETY: JavaScriptCore owns the exception out-pointer when it is
            // non-null; writing null marks successful completion.
            unsafe { *#exception_ident = std::ptr::null_mut() };
        }
    }
}

fn write_exception(exception_ident: &syn::Ident, value: TokenStream2) -> TokenStream2 {
    quote! {
        if !#exception_ident.is_null() {
            // SAFETY: JavaScriptCore owns the exception out-pointer when it is
            // non-null; the generated wrapper transfers the thrown JSValueRef.
            unsafe { *#exception_ident = #value };
        }
    }
}

fn generate_optional_param_parsing(
    idx: syn::Index,
    var_ident: &syn::Ident,
    failure_return: TokenStream2,
) -> TokenStream2 {
    let exception_ident = syn::Ident::new("__exception", Span::call_site());
    let write_err = write_exception(
        &exception_ident,
        quote!(rust_jsc::internal::JSValueRef::from(err) as *mut _),
    );

    quote! {
        let #var_ident = match __raw_arguments.get(#idx) {
            Some(__inner_value) => {
                // SAFETY: JavaScriptCore provided this argument value as
                // belonging to `__ctx_ref` for the callback duration.
                let __value =
                    unsafe { rust_jsc::JSValue::from_raw_unchecked(*__inner_value, __ctx_ref) };
                match rust_jsc::TryFromJSValue::try_from_js_value(&__value) {
                    Ok(value) => value,
                    Err(err) => {
                        #write_err
                        return #failure_return;
                    }
                }
            }
            None => None,
        };
    }
}

fn generate_required_param_parsing(
    idx: syn::Index,
    var_ident: &syn::Ident,
    fn_name: &str,
    param_name: &str,
    failure_return: TokenStream2,
) -> TokenStream2 {
    let exception_ident = syn::Ident::new("__exception", Span::call_site());
    let write_conversion_err = write_exception(
        &exception_ident,
        quote!(rust_jsc::internal::JSValueRef::from(err) as *mut _),
    );
    let write_missing_err = write_exception(
        &exception_ident,
        quote!(rust_jsc::JSError::new_typ_raw(
            &ctx,
            format!("[{}] Missing argument {}", #fn_name, #param_name)
        ) as *mut _),
    );

    quote! {
        let #var_ident = match __raw_arguments.get(#idx) {
            Some(__inner_value) => {
                // SAFETY: JavaScriptCore provided this argument value as
                // belonging to `__ctx_ref` for the callback duration.
                let __value =
                    unsafe { rust_jsc::JSValue::from_raw_unchecked(*__inner_value, __ctx_ref) };
                match rust_jsc::TryFromJSValue::try_from_js_value(&__value) {
                    Ok(value) => value,
                    Err(err) => {
                        #write_conversion_err
                        return #failure_return;
                    }
                }
            }
            None => {
                #write_missing_err
                return #failure_return;
            },
        };
    }
}

fn generate_rest_param_parsing(
    idx: syn::Index,
    var_ident: &syn::Ident,
    failure_return: TokenStream2,
) -> TokenStream2 {
    let exception_ident = syn::Ident::new("__exception", Span::call_site());
    let write_err = write_exception(
        &exception_ident,
        quote!(rust_jsc::internal::JSValueRef::from(err) as *mut _),
    );

    quote! {
        let mut __rest_values = std::vec::Vec::with_capacity(
            __raw_arguments.len().saturating_sub(#idx),
        );
        for __inner_value in __raw_arguments.iter().skip(#idx) {
            // SAFETY: JavaScriptCore provided this argument value as belonging
            // to `__ctx_ref` for the callback duration.
            let __value =
                unsafe { rust_jsc::JSValue::from_raw_unchecked(*__inner_value, __ctx_ref) };
            match rust_jsc::TryFromJSValue::try_from_js_value(&__value) {
                Ok(value) => __rest_values.push(value),
                Err(err) => {
                    #write_err
                    return #failure_return;
                }
            }
        }
        let #var_ident = rust_jsc::Rest::from_vec(__rest_values);
    }
}

fn is_option_type(ty: &Type) -> bool {
    match strip_group(ty) {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|s| s.ident == "Option")
            .unwrap_or(false),
        _ => false,
    }
}

fn is_rest_type(ty: &Type) -> bool {
    match strip_group(ty) {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|s| s.ident == "Rest")
            .unwrap_or(false),
        _ => false,
    }
}

fn option_inner_ident(ty: &Type) -> Option<&syn::Ident> {
    let Type::Path(TypePath { path, .. }) = strip_group(ty) else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };

    if args.len() != 1 {
        return None;
    }

    let Some(GenericArgument::Type(inner)) = args.first() else {
        return None;
    };

    type_path_last_ident(inner)
}

fn is_js_value_slice_reference(ty: &Type) -> bool {
    let Type::Reference(reference) = strip_group(ty) else {
        return false;
    };

    let Type::Slice(slice) = strip_group(&reference.elem) else {
        return false;
    };

    type_path_last_ident(&slice.elem).is_some_and(|ident| ident == "JSValue")
}

fn is_str_reference(ty: &Type) -> bool {
    let Type::Reference(reference) = strip_group(ty) else {
        return false;
    };

    type_path_last_ident(&reference.elem).is_some_and(|ident| ident == "str")
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(strip_group(ty), Type::Tuple(tuple) if tuple.elems.is_empty())
}

fn type_path_last_ident(ty: &Type) -> Option<&syn::Ident> {
    let Type::Path(TypePath { path, .. }) = strip_group(ty) else {
        return None;
    };

    let segment = path.segments.last()?;
    if !matches!(segment.arguments, PathArguments::None)
        && segment.ident != "JSResult"
        && segment.ident != "Option"
    {
        return None;
    }

    Some(&segment.ident)
}

fn js_result_inner_ident(ty: &Type) -> Option<&syn::Ident> {
    let Type::Path(TypePath { path, .. }) = strip_group(ty) else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "JSResult" {
        return None;
    }

    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };

    if args.len() != 1 {
        return None;
    }

    let Some(GenericArgument::Type(inner)) = args.first() else {
        return None;
    };

    type_path_last_ident(inner)
}

fn strip_group(ty: &Type) -> &Type {
    match ty {
        Type::Group(group) => strip_group(&group.elem),
        Type::Paren(paren) => strip_group(&paren.elem),
        _ => ty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fn(source: &str) -> ItemFn {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn detects_exact_legacy_callback_raw_slice() {
        let input = parse_fn(
            r#"
            fn callback(
                ctx: JSContext,
                function: JSObject,
                this: JSObject,
                args: &[JSValue],
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
        assert!(matches!(
            callback_arguments(&input).unwrap(),
            MacroArguments::LegacyRawSlice { .. }
        ));
    }

    #[test]
    fn rejects_non_slice_reference_callback_arg() {
        let input = parse_fn(
            r#"
            fn callback(
                ctx: JSContext,
                function: JSObject,
                this: JSObject,
                arg: &str,
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
        assert!(callback_arguments(&input).is_err());
    }

    #[test]
    fn validates_callback_without_abi_prefix() {
        let input = parse_fn(
            r#"
            fn callback(left: f64, right: f64) -> f64 { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
        assert!(matches!(
            callback_arguments(&input).unwrap(),
            MacroArguments::Typed { .. }
        ));
    }

    #[test]
    fn validates_callback_injection_markers() {
        let input = parse_fn(
            r#"
            fn callback(
                ctx: CallbackContext,
                this: ThisObject,
                function: CallbackFunction,
                value: f64,
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
        assert!(matches!(
            callback_arguments(&input).unwrap(),
            MacroArguments::Typed { .. }
        ));
    }

    #[test]
    fn rejects_callback_context_outside_legacy_prefix() {
        let input = parse_fn(
            r#"
            fn callback(ctx: JSContext, value: f64) -> JSResult<JSValue> { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::Callback).is_err());
    }

    #[test]
    fn rejects_callback_injection_after_js_argument() {
        let input = parse_fn(
            r#"
            fn callback(value: f64, this: ThisObject) -> JSResult<JSValue> { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::Callback).is_err());
    }

    #[test]
    fn validates_constructor_shape() {
        let input = parse_fn(
            r#"
            fn constructor(
                ctx: JSContext,
                this: JSObject,
                args: &[JSValue],
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Constructor).unwrap();
        assert!(matches!(
            constructor_arguments(&input).unwrap(),
            MacroArguments::LegacyRawSlice { .. }
        ));
    }

    #[test]
    fn validates_constructor_injection_markers() {
        let input = parse_fn(
            r#"
            fn constructor(
                ctx: CallbackContext,
                constructor: ConstructorObject,
                name: String,
            ) -> JSResult<JSObject> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Constructor).unwrap();
        assert!(matches!(
            constructor_arguments(&input).unwrap(),
            MacroArguments::Typed { .. }
        ));
    }

    #[test]
    fn rejects_constructor_context_outside_legacy_prefix() {
        let input = parse_fn(
            r#"
            fn constructor(ctx: JSContext, name: String) -> JSResult<JSObject> { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::Constructor).is_err());
    }

    #[test]
    fn validates_typed_constructor_shape() {
        let input = parse_fn(
            r#"
            fn constructor(
                ctx: JSContext,
                this: JSObject,
                name: String,
                value: Option<f64>,
                rest: Rest<JSValue>,
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Constructor).unwrap();
        assert!(matches!(
            constructor_arguments(&input).unwrap(),
            MacroArguments::Typed { .. }
        ));
    }

    #[test]
    fn validates_initialize_shape() {
        let input = parse_fn(
            r#"
            fn initialize(ctx: JSContext, object: JSObject) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::Initialize).unwrap();
    }

    #[test]
    fn validates_finalize_shape() {
        let input = parse_fn(
            r#"
            fn finalize(data: PrivateData) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::Finalize).unwrap();
    }

    #[test]
    fn rejects_rest_argument_before_final_position() {
        let input = parse_fn(
            r#"
            fn callback(
                ctx: JSContext,
                function: JSObject,
                this: JSObject,
                rest: Rest<JSValue>,
                next: JSValue,
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
        assert!(callback_arguments(&input).is_err());
    }

    #[test]
    fn rejects_async_callback_shape() {
        let input = parse_fn(
            r#"
            async fn callback(
                ctx: JSContext,
                function: JSObject,
                this: JSObject,
            ) -> JSResult<JSValue> { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::Callback).is_err());
    }

    #[test]
    fn accepts_typed_callback_return_type() {
        let input = parse_fn(
            r#"
            fn callback(
                ctx: JSContext,
                function: JSObject,
                this: JSObject,
            ) -> JSResult<bool> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::Callback).unwrap();
    }

    #[test]
    fn validates_module_resolve_shape() {
        let input = parse_fn(
            r#"
            fn resolve(
                ctx: JSContext,
                key: JSValue,
                referrer: JSValue,
                script_fetcher: JSValue,
            ) -> JSStringProtected { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleResolve).unwrap();
    }

    #[test]
    fn validates_module_evaluate_shape() {
        let input = parse_fn(
            r#"
            fn evaluate(ctx: JSContext, key: JSValue) -> JSValue { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleEvaluate).unwrap();
    }

    #[test]
    fn rejects_module_fetch_wrong_return_type() {
        let input = parse_fn(
            r#"
            fn fetch(
                ctx: JSContext,
                key: JSValue,
                attributes: JSValue,
                script_fetcher: JSValue,
            ) -> JSValue { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::ModuleFetch).is_err());
    }

    #[test]
    fn validates_module_import_meta_shape() {
        let input = parse_fn(
            r#"
            fn import_meta(
                ctx: JSContext,
                key: JSValue,
                script_fetcher: JSValue,
            ) -> JSObject { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleImportMeta).unwrap();
    }

    #[test]
    fn validates_typed_module_resolver_shape() {
        let input = parse_fn(
            r#"
            fn resolve(
                ctx: JSContext,
                specifier: String,
                referrer: Option<String>,
            ) -> JSResult<Option<String>> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleResolver).unwrap();
    }

    #[test]
    fn rejects_typed_module_resolver_wrong_referrer_type() {
        let input = parse_fn(
            r#"
            fn resolve(
                ctx: JSContext,
                specifier: String,
                referrer: JSValue,
            ) -> JSResult<Option<String>> { todo!() }
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::ModuleResolver).is_err());
    }

    #[test]
    fn validates_typed_module_fetcher_shape() {
        let input = parse_fn(
            r#"
            fn fetch(
                ctx: JSContext,
                key: String,
                import_type: ModuleImportType,
            ) -> JSResult<Option<ModuleSource>> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleFetcher).unwrap();
    }

    #[test]
    fn validates_typed_module_import_meta_provider_shape() {
        let input = parse_fn(
            r#"
            fn import_meta(ctx: JSContext, key: String) -> JSResult<JSObject> { todo!() }
            "#,
        );

        validate_abi_role(&input, AbiRole::ModuleImportMetaProvider).unwrap();
    }

    #[test]
    fn validates_inspector_callback_shape() {
        let input = parse_fn(
            r#"
            fn inspector(message: &str) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::InspectorCallback).unwrap();
    }

    #[test]
    fn validates_inspector_callback_explicit_unit_return() {
        let input = parse_fn(
            r#"
            fn inspector(message: &str) -> () {}
            "#,
        );

        validate_abi_role(&input, AbiRole::InspectorCallback).unwrap();
    }

    #[test]
    fn rejects_inspector_callback_owned_string() {
        let input = parse_fn(
            r#"
            fn inspector(message: String) {}
            "#,
        );

        assert!(validate_abi_role(&input, AbiRole::InspectorCallback).is_err());
    }

    #[test]
    fn validates_uncaught_exception_shape() {
        let input = parse_fn(
            r#"
            fn uncaught(ctx: JSContext, filename: JSString, exception: JSValue) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::UncaughtException).unwrap();
    }

    #[test]
    fn validates_uncaught_exception_event_loop_shape() {
        let input = parse_fn(
            r#"
            fn uncaught_event_loop(ctx: JSContext, exception: JSValue) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::UncaughtExceptionEventLoop).unwrap();
    }

    #[test]
    fn validates_inspector_pause_event_callback_shape() {
        let input = parse_fn(
            r#"
            fn pause(ctx: JSContext, event: InspectorPauseEvent) {}
            "#,
        );

        validate_abi_role(&input, AbiRole::InspectorPauseEventCallback).unwrap();
    }
}
