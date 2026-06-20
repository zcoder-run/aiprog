extern crate proc_macro;

mod aip_error;
mod aip_from_lua;
mod aip_into_lua;
mod aip_params;
mod aip_response;

#[proc_macro_derive(AipFromLua)]
pub fn aip_from_lua_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	aip_from_lua::aip_from_lua_derive(input)
}

#[proc_macro_derive(AipIntoLua)]
pub fn aip_into_lua_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	aip_into_lua::aip_into_lua_derive(input)
}

#[proc_macro_derive(AipParams)]
pub fn aip_params_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	aip_params::aip_params_derive(input)
}

#[proc_macro_derive(AipResponse)]
pub fn aip_response_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	aip_response::aip_response_derive(input)
}

#[proc_macro_derive(AipError)]
pub fn aip_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	aip_error::aip_error_derive(input)
}

// region:    --- Tests

#[cfg(test)]
#[path = "tests_derive.rs"]
mod tests_derive;

// endregion: --- Tests
