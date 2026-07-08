extern crate proc_macro;

mod aip_handler;
mod derive;
mod register_handler;

// region:    --- Derives

#[proc_macro_derive(AipFromLua)]
pub fn aip_from_lua_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive::aip_from_lua_derive(input)
}

#[proc_macro_derive(AipIntoLua)]
pub fn aip_into_lua_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive::aip_into_lua_derive(input)
}

#[proc_macro_derive(AipParams)]
pub fn aip_params_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive::aip_params_derive(input)
}

#[proc_macro_derive(AipOutput)]
pub fn aip_output_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive::aip_output_derive(input)
}

#[proc_macro_derive(AipError)]
pub fn aip_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive::aip_error_derive(input)
}

// endregion: --- Derives

// region:    --- Attributes

#[proc_macro_attribute]
pub fn aip_handler(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let attr: proc_macro2::TokenStream = attr.into();
	let item: proc_macro2::TokenStream = item.into();
	let result = aip_handler::aip_handler_attr(attr, item);
	result.into()
}

// endregion: --- Attributes

// region:    --- Function-like macros

#[proc_macro]
pub fn register_handler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	register_handler::register_handler_impl(input)
}

// endregion: --- Function-like macros
