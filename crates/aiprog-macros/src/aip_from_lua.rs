use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn aip_from_lua_derive(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = &input.ident;
	let generics = add_serde_bounds(input.generics);
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let expanded = quote! {
		impl #impl_generics ::aiprog::script::AipFromLua for #ident #ty_generics #where_clause {
			fn from_lua(_lua: &::aiprog::mlua::Lua, value: ::aiprog::mlua::Value) -> ::aiprog::script::ScriptResult<Self> {
				let serde_value = ::aiprog::script::LuaJsonExt::x_to_json_value(&value)
					.map_err(|e| ::aiprog::ScriptError::custom(format!("Invalid params: {e}")))?;
				let serde_value = serde_value
					.ok_or_else(|| ::aiprog::ScriptError::custom("expected JSON value, got nil".to_string()))?;
				Ok(::aiprog::serde_json::from_value(serde_value)
					.map_err(|e| format!("deserialization error: {e}"))?)
			}
		}
	};

	TokenStream::from(expanded)
}

fn add_serde_bounds(mut generics: syn::Generics) -> syn::Generics {
	for param in &mut generics.params {
		if let syn::GenericParam::Type(ref mut type_param) = *param {
			type_param.bounds.push(syn::parse_quote!(::serde::Serialize));
			type_param.bounds.push(syn::parse_quote!(::serde::de::DeserializeOwned));
		}
	}
	generics
}
