extern crate proc_macro;

mod derive;

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
