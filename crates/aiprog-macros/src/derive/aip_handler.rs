use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub fn aip_handler_attr(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let input = parse_macro_input!(item as ItemFn);

	// Extract doc comment metadata.
	let (title, desc) = extract_title_desc(&input.attrs);

	let is_async = input.sig.asyncness.is_some();
	// All handlers (sync and async) must have exactly one typed parameter.
	{
		let param_count = input
			.sig
			.inputs
			.iter()
			.filter(|arg| matches!(arg, syn::FnArg::Typed(_)))
			.count();
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
	// Partition attributes: doc attrs are consumed as metadata; everything else
	// stays on the function.
	let (_struct_attrs, impl_attrs): (Vec<_>, Vec<_>) =
		input.attrs.iter().cloned().partition(|attr| attr.path().is_ident("doc"));

	// The output function is the original function, keeping only non-doc attributes.
	// The hidden function keeps non-doc attributes, renamed to avoid collision with the struct.
	let original_ident = input.sig.ident.clone();
	let hidden_ident = syn::Ident::new(&format!("__aiprog_{}", original_ident), original_ident.span());
	let mut hidden_fn = input.clone();
	hidden_fn.attrs = impl_attrs;
	hidden_fn.sig.ident = hidden_ident.clone();

	// Extract trait associated types.
	let params_ty = if is_async {
		get_params_ty_async(&input.sig)
	} else {
		get_params_ty_sync(&input.sig)
	};
	let output_ty = get_output_inner_type(&input.sig);

	let marker = if is_async {
		quote! { ::aiprog::registry::handler_types::AsyncMarker }
	} else {
		quote! { ::aiprog::registry::handler_types::SyncMarker }
	};

	let desc_owned_tokens = if let Some(s) = &desc {
		let lit = syn::LitStr::new(s, original_ident.span());
		quote! { ::core::option::Option::Some(#lit.to_string()) }
	} else {
		quote! { ::core::option::Option::None }
	};
	let title_owned_tokens = if let Some(s) = &title {
		let lit = syn::LitStr::new(s, original_ident.span());
		quote! { ::core::option::Option::Some(#lit.to_string()) }
	} else {
		quote! { ::core::option::Option::None }
	};

	let meta_fn_ident = syn::Ident::new(&format!("__aiprog_meta_{}", original_ident), original_ident.span());
	let meta_fn = quote! {
		fn #meta_fn_ident() -> ::aiprog::registry::AipHandlerMeta {
			::aiprog::registry::AipHandlerMeta {
				description: #desc_owned_tokens,
				title: #title_owned_tokens,
			}
		}
	};

	let struct_def = quote! {
		#[allow(non_camel_case_types)]
		struct #original_ident;
	};

	let impl_block = if is_async {
		quote! {
			impl ::aiprog::registry::AipHandler for #original_ident
			where
				#output_ty : serde::Serialize,
			{
				type Marker = #marker;
				type Params = #params_ty;
				type Output = #output_ty;

				fn handler_meta() -> ::aiprog::registry::AipHandlerMeta {
					#meta_fn_ident()
				}

				fn create_entry(path: &str) -> ::aiprog::registry::registry_internal::RegistryEntry {
					let params_schema = schemars::schema_for!(#params_ty);
					let output_schema = schemars::schema_for!(#output_ty);
					let error_schema = schemars::schema_for!(::aiprog::HandlerError);

					let closure: ::aiprog::registry::registry_internal::LuaAsyncClosure = Box::new(move |lua: &mlua::Lua, value: mlua::Value| {
						let params = match <#params_ty as ::aiprog::AipFromLua>::from_lua(lua, value)
							.map_err(|e| mlua::Error::ExternalError(::std::sync::Arc::new(e))) {
							Ok(p) => p,
							Err(e) => return Box::pin(async move { Err(e) }),
						};

						Box::pin(async move {
							match #hidden_ident(params).await {
								Ok(output) => serde_json::to_value(output)
									.map_err(|e| mlua::Error::RuntimeError(format!("Failed to serialize async response: {e}"))),
								Err(e) => Err(e.into_lua_error()),
							}
						})
					});

					let meta = Self::handler_meta();
					::aiprog::registry::registry_internal::RegistryEntry {
						path: path.to_string(),
						kind: ::aiprog::registry::AipFnKind::Async,
						handler: ::aiprog::registry::registry_internal::AipHandlerClosure::Async(closure),
						params_schema,
						output_schema,
						error_schema,
						description: meta.description,
						title: meta.title,
					}
				}
			}
		}
	} else {
		quote! {
			impl ::aiprog::registry::AipHandler for #original_ident {
				type Marker = #marker;
				type Params = #params_ty;
				type Output = #output_ty;

				fn handler_meta() -> ::aiprog::registry::AipHandlerMeta {
					#meta_fn_ident()
				}

				fn create_entry(path: &str) -> ::aiprog::registry::registry_internal::RegistryEntry {
					let params_schema = schemars::schema_for!(#params_ty);
					let output_schema = schemars::schema_for!(#output_ty);
					let error_schema = schemars::schema_for!(::aiprog::HandlerError);

					let closure: ::aiprog::registry::registry_internal::LuaSyncClosure = Box::new(move |lua, value| {
						let params = <#params_ty as ::aiprog::AipFromLua>::from_lua(&lua, value)
							.map_err(|e| mlua::Error::ExternalError(std::sync::Arc::new(e)))?;
						let result = #hidden_ident(params);
						match result {
							Ok(output) => {
								<#output_ty as ::aiprog::AipIntoLua>::into_lua(output, &lua)
									.map_err(|e| mlua::Error::ExternalError(std::sync::Arc::new(e)))
							}
							Err(e) => {
								let err: ::aiprog::Error = e.into();
								Err(mlua::Error::ExternalError(std::sync::Arc::new(err)))
							}
						}
					});

					let meta = Self::handler_meta();
					::aiprog::registry::registry_internal::RegistryEntry {
						path: path.to_string(),
						kind: ::aiprog::registry::AipFnKind::Sync,
						handler: ::aiprog::registry::registry_internal::AipHandlerClosure::Sync(closure),
						params_schema,
						output_schema,
						error_schema,
						description: meta.description,
						title: meta.title,
					}
				}
			}
		}
	};

	let expanded = quote! {
		#hidden_fn
		#meta_fn
		#struct_def
		#impl_block
	};

	TokenStream::from(expanded)
}

// region:    --- Helpers

fn extract_title_desc(attrs: &[syn::Attribute]) -> (Option<String>, Option<String>) {
	let doc_lines: Vec<String> = attrs
		.iter()
		.filter(|a| a.path().is_ident("doc"))
		.filter_map(|a| {
			if let syn::Meta::NameValue(nv) = &a.meta
				&& let syn::Expr::Lit(el) = &nv.value
				&& let syn::Lit::Str(s) = &el.lit
			{
				Some(s.value())
			} else {
				None
			}
		})
		.collect();
	let doc_str = doc_lines.join("\n");
	let doc_str = doc_str.trim();
	if doc_str.is_empty() {
		return (None, None);
	}

	let first_line = doc_str.lines().find(|l| !l.trim().is_empty()).map(|l| l.trim());
	if let Some(first) = first_line
		&& let Some(stripped) = first.strip_prefix("# ")
	{
		let title = stripped.trim().to_string();
		let idx = doc_str.lines().position(|l| l.trim() == first).unwrap();
		let rest = doc_str.lines().skip(idx + 1).collect::<Vec<_>>().join("\n").trim().to_string();
		let desc = if rest.is_empty() { None } else { Some(rest) };
		return (Some(title), desc);
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
	if let syn::Type::Path(type_path) = &output
		&& let Some(segment) = type_path.path.segments.last()
		&& segment.ident == "HandlerResult"
		&& let syn::PathArguments::AngleBracketed(args) = &segment.arguments
		&& let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
	{
		return inner_ty.clone();
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
		let output_str = result.to_string();
		assert!(output_str.contains("impl crate::registry::AipHandler for my_parse_handler"));
		assert!(output_str.contains("struct"));
		assert!(output_str.contains("__aiprog_meta_my_parse_handler"));
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
		let output_str = result.to_string();
		assert!(output_str.contains("AsyncMarker"));
		assert!(output_str.contains("struct"));
		assert!(output_str.contains("__aiprog_meta_my_async_handler"));
	}
}

// endregion: --- Tests
