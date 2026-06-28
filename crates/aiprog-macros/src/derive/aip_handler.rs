use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Visibility};

pub fn aip_handler_attr(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    // Extract doc comment metadata.
    let (title, desc) = extract_title_desc(&input.attrs);

    let is_async = input.sig.asyncness.is_some();
    // All handlers (sync and async) must have exactly one typed parameter.
    {
        let param_count = input.sig.inputs.iter().filter(|arg| matches!(arg, syn::FnArg::Typed(_))).count();
        if param_count != 1 {
            return syn::Error::new_spanned(
                &input.sig,
                "handler function must have exactly one parameter (the typed params).",
            )
            .to_compile_error()
            .into();
        }
    }

    // Partition attributes: doc attrs → struct; everything else → impl fn.
    let (struct_attrs, impl_attrs): (Vec<_>, Vec<_>) = input
        .attrs
        .iter()
        .cloned()
        .partition(|attr| attr.path().is_ident("doc"));

    // Build the private implementation function by renaming the original.
    let mut impl_fn = input.clone();
    impl_fn.attrs = impl_attrs;
    impl_fn.vis = Visibility::Inherited;
    let original_ident = impl_fn.sig.ident.clone();
    let impl_fn_ident = syn::Ident::new(
        &format!("__{}_impl", original_ident),
        original_ident.span(),
    );
    impl_fn.sig.ident = impl_fn_ident.clone();

    let struct_vis = input.vis.clone();
    let struct_name = original_ident;

    // Extract trait associated types.
    let params_ty = if is_async {
        get_params_ty_async(&input.sig)
    } else {
        get_params_ty_sync(&input.sig)
    };
    let output_ty = get_output_inner_type(&input.sig);

    // Build static string literal tokens for title/description.
    let desc_tokens = if let Some(s) = &desc {
        let lit = syn::LitStr::new(s, struct_name.span());
        quote! { ::core::option::Option::Some(#lit) }
    } else {
        quote! { ::core::option::Option::None }
    };
    let title_tokens = if let Some(s) = &title {
        let lit = syn::LitStr::new(s, struct_name.span());
        quote! { ::core::option::Option::Some(#lit) }
    } else {
        quote! { ::core::option::Option::None }
    };

    let marker = if is_async {
        quote! { crate::registry::handler_types::AsyncMarker }
    } else {
        quote! { crate::registry::handler_types::SyncMarker }
    };

    let expanded = if is_async {
        quote! {
            #impl_fn

            #( #struct_attrs )*
            #struct_vis struct #struct_name;

            impl crate::registry::AipHandler for #struct_name
            where
                #output_ty : serde::Serialize,
            {
                type Marker = #marker;
                type Params = #params_ty;
                type Output = #output_ty;

                fn handler_desc() -> Option<&'static str> {
                    #desc_tokens
                }
                fn handler_title() -> Option<&'static str> {
                    #title_tokens
                }

                fn create_entry(path: &str) -> crate::registry::registry_internal::RegistryEntry {
                    let params_schema = schemars::schema_for!(#params_ty);
                    let output_schema = schemars::schema_for!(#output_ty);
                    let error_schema = schemars::schema_for!(crate::HandlerError);

                    let closure: crate::registry::registry_internal::LuaAsyncClosure = Box::new(move |lua: &mlua::Lua, value: mlua::Value| {
                        let params = match <#params_ty as crate::AipFromLua>::from_lua(lua, value)
                            .map_err(|e| mlua::Error::ExternalError(::std::sync::Arc::new(e))) {
                            Ok(p) => p,
                            Err(e) => return Box::pin(async move { Err(e) }),
                        };

                        Box::pin(async move {
                            match #impl_fn_ident(params).await {
                                Ok(output) => serde_json::to_value(output)
                                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to serialize async response: {e}"))),
                                Err(e) => Err(e.into_lua_error()),
                            }
                        })
                    });

                    crate::registry::registry_internal::RegistryEntry {
                        path: path.to_string(),
                        kind: crate::registry::AipFnKind::Async,
                        handler: crate::registry::registry_internal::AipHandlerClosure::Async(closure),
                        params_schema,
                        output_schema,
                        error_schema,
                        description: Self::handler_desc().map(|s| s.to_string()),
                        title: Self::handler_title().map(|s| s.to_string()),
                    }
                }
            }
        }
    } else {
        quote! {
            #impl_fn

            #( #struct_attrs )*
            #struct_vis struct #struct_name;

            impl crate::registry::AipHandler for #struct_name {
                type Marker = #marker;
                type Params = #params_ty;
                type Output = #output_ty;

                fn handler_desc() -> Option<&'static str> {
                    #desc_tokens
                }
                fn handler_title() -> Option<&'static str> {
                    #title_tokens
                }

                fn create_entry(path: &str) -> crate::registry::registry_internal::RegistryEntry {
                    let params_schema = schemars::schema_for!(#params_ty);
                    let output_schema = schemars::schema_for!(#output_ty);
                    let error_schema = schemars::schema_for!(crate::HandlerError);

                    let closure: crate::registry::registry_internal::LuaSyncClosure = Box::new(move |lua, value| {
                        let params = <#params_ty as crate::AipFromLua>::from_lua(&lua, value)
                            .map_err(|e| mlua::Error::ExternalError(std::sync::Arc::new(e)))?;
                        let result = #impl_fn_ident(params);
                        match result {
                            Ok(output) => {
                                <#output_ty as crate::AipIntoLua>::into_lua(output, &lua)
                                    .map_err(|e| mlua::Error::ExternalError(std::sync::Arc::new(e)))
                            }
                            Err(e) => {
                                let err: crate::Error = e.into();
                                Err(mlua::Error::ExternalError(std::sync::Arc::new(err)))
                            }
                        }
                    });

                    crate::registry::registry_internal::RegistryEntry {
                        path: path.to_string(),
                        kind: crate::registry::AipFnKind::Sync,
                        handler: crate::registry::registry_internal::AipHandlerClosure::Sync(closure),
                        params_schema,
                        output_schema,
                        error_schema,
                        description: Self::handler_desc().map(|s| s.to_string()),
                        title: Self::handler_title().map(|s| s.to_string()),
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// region:    --- Helpers

fn extract_title_desc(attrs: &[syn::Attribute]) -> (Option<String>, Option<String>) {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| {
            if let syn::Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(s) = &el.lit {
                        return Some(s.value());
                    }
                }
            }
            None
        })
        .collect();
    let doc_str = doc_lines.join("\n");
    let doc_str = doc_str.trim();
    if doc_str.is_empty() {
        return (None, None);
    }

    let first_line = doc_str
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim());

    if let Some(first) = first_line {
        if first.starts_with("# ") {
            let title = first[2..].trim().to_string();
            let idx = doc_str.lines().position(|l| l.trim() == first).unwrap();
            let rest = doc_str
                .lines()
                .skip(idx + 1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            let desc = if rest.is_empty() { None } else { Some(rest) };
            return (Some(title), desc);
        }
    }

    (None, Some(doc_str.to_string()))
}

fn get_params_ty_sync(sig: &syn::Signature) -> syn::Type {
    let first = sig
        .inputs
        .iter()
        .find_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => Some(pat),
        })
        .expect("handler function must have exactly one typed parameter (the typed params)");
    (*first.ty).clone()
}

fn get_params_ty_async(sig: &syn::Signature) -> syn::Type {
    let first = sig
        .inputs
        .iter()
        .find_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => Some(pat),
        })
        .expect("async handler must have exactly one typed parameter (the typed params)");
    (*first.ty).clone()
}

fn get_output_inner_type(sig: &syn::Signature) -> syn::Type {
    let output = match &sig.output {
        syn::ReturnType::Default => panic!("handler function must specify a return type"),
        syn::ReturnType::Type(_, ty) => ty.as_ref().clone(),
    };
    if let syn::Type::Path(type_path) = &output {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "HandlerResult" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return inner_ty.clone();
                    }
                }
            }
        }
    }
    panic!("handler function must return `HandlerResult<T>`");
}

// endregion: --- Helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_sync_handler_expands() {
        let input = quote! {
            /// Parses a JSON string and returns the parsed value.
            fn my_parse_handler(
                params: super::MyParams,
            ) -> ::aiprog::HandlerResult<super::MyOutput> {
                unimplemented!()
            }
        };
        let result = aip_handler_attr(TokenStream::new(), input.into());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_async_handler_expands() {
        let input = quote! {
            /// An async handler that processes data.
            async fn my_async_handler(
                params: super::MyParams,
            ) -> ::aiprog::HandlerResult<super::MyOutput> {
                unimplemented!()
            }
        };
        let result = aip_handler_attr(TokenStream::new(), input.into());
        assert!(!result.is_empty());
        let output_str = result.to_string();
        assert!(output_str.contains("AsyncMarker"));
    }
}

// endregion: --- Tests
