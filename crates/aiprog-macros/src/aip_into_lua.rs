use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn aip_into_lua_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let generics = add_serialize_bound(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::aiprog::script::AipIntoLua for #ident #ty_generics #where_clause {
            fn into_lua(self, lua: &::aiprog::mlua::Lua) -> ::aiprog::script::ScriptResult<::aiprog::mlua::Value> {
                let serde_value = ::aiprog::serde_json::to_value(self)
                    .map_err(|e| ::aiprog::ScriptError::custom(e.to_string()))?;
                <::aiprog::mlua::Value as ::aiprog::script::LuaJsonExt>::x_from_json_value(lua, serde_value)
            }
        }
    };

    TokenStream::from(expanded)
}

fn add_serialize_bound(mut generics: syn::Generics) -> syn::Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(syn::parse_quote!(::serde::Serialize));
        }
    }
    generics
}
