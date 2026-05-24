use crate::script::helpers::{lua_value_to_serde_value, serde_value_to_lua_value};
use mlua::{Lua, Value};
use serde::de::DeserializeOwned;

// region:    --- Params

pub fn lua_params_from_value<T>(arg: Value) -> mlua::Result<T>
where
	T: DeserializeOwned,
{
	let json_value = lua_value_to_serde_value(arg)
		.map_err(|err| aip_error("INVALID_PARAMS", Some(err.to_string()), None))?;
	serde_json::from_value(json_value).map_err(|err| aip_error("INVALID_PARAMS", Some(err.to_string()), None))
}

// endregion: --- Params

// region:    --- Response

pub fn return_success_envelope<T: serde::Serialize>(lua: &Lua, data: T) -> mlua::Result<Value> {
	let serde_val = serde_json::to_value(&data)
		.map_err(|err| mlua::Error::RuntimeError(format!("Failed to serialize API success response: {err}")))?;
	serde_value_to_lua_value(lua, serde_val).map_err(Into::into)
}

pub fn return_error_envelope(
	_lua: &Lua,
	message: &str,
	full_message: Option<String>,
	cause: Option<String>,
) -> mlua::Result<Value> {
	Err(aip_error(message, full_message, cause))
}

// endregion: --- Response

// region:    --- Support

fn aip_error(message: &str, full_message: Option<String>, cause: Option<String>) -> mlua::Error {
	let mut err_msg = format!("API Error: {message}");
	if let Some(fm) = full_message {
		err_msg.push_str(&format!("\nDetails: {fm}"));
	}
	if let Some(c) = cause {
		err_msg.push_str(&format!("\nCause: {c}"));
	}
	mlua::Error::RuntimeError(err_msg)
}

// endregion: --- Support
