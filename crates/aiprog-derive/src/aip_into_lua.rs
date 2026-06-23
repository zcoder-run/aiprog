use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn aip_into_lua_derive(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = &input.ident;

	let is_single_field_tuple = is_single_field_tuple(&input.data);

	let generics = if is_single_field_tuple {
		add_trait_bound(input.generics, syn::parse_quote!(::aiprog::AipIntoLua))
	} else {
		add_trait_bound(input.generics, syn::parse_quote!(::serde::Serialize))
	};
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let body = if is_single_field_tuple {
		quote! {
			fn into_lua(self, lua: &::aiprog::mlua::Lua) -> ::aiprog::ScriptResult<::aiprog::mlua::Value> {
				self.0.into_lua(lua)
			}
		}
	} else {
		quote! {
			fn into_lua(self, lua: &::aiprog::mlua::Lua) -> ::aiprog::ScriptResult<::aiprog::mlua::Value> {
				let serde_value = ::aiprog::serde_json::to_value(self)
					.map_err(|e| ::aiprog::ScriptError::custom(e.to_string()))?;
				<::aiprog::mlua::Value as ::aiprog::LuaJsonExt>::x_from_json_value(lua, serde_value)
			}
		}
	};

	let expanded = quote! {
		impl #impl_generics ::aiprog::AipIntoLua for #ident #ty_generics #where_clause {
			#body
		}
	};

	TokenStream::from(expanded)
}

fn is_single_field_tuple(data: &syn::Data) -> bool {
	matches!(data, syn::Data::Struct(s) if matches!(&s.fields, syn::Fields::Unnamed(u) if u.unnamed.len() == 1))
}

fn add_trait_bound(mut generics: syn::Generics, bound: syn::TypeParamBound) -> syn::Generics {
	for param in &mut generics.params {
		if let syn::GenericParam::Type(ref mut type_param) = *param {
			type_param.bounds.push(bound.clone());
		}
	}
	generics
}
