use crate::script::helpers::{lua_value_to_serde_value, serde_value_to_lua_value};
use mlua::{Lua, Value};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

// region:    --- AipFn Error

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipApiError {
	pub code: String,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cause: Option<String>,
}

pub trait IntoAipLuaError {
	fn into_aip_lua_error(self) -> mlua::Error;
}

impl IntoAipLuaError for AipApiError {
	fn into_aip_lua_error(self) -> mlua::Error {
		let mut err_msg = format!("API Error [{}]: {}", self.code, self.message);
		if let Some(details) = &self.details {
			err_msg.push_str(&format!("\nDetails: {details}"));
		}
		if let Some(cause) = &self.cause {
			err_msg.push_str(&format!("\nCause: {cause}"));
		}
		mlua::Error::RuntimeError(err_msg)
	}
}

// endregion: --- AipFn Error

// region:    --- Function

pub trait AipFn {
	const NAME: &'static str;

	type Params: serde::de::DeserializeOwned + schemars::JsonSchema;
	type Response: serde::Serialize + schemars::JsonSchema;
	type Error: serde::Serialize + schemars::JsonSchema + IntoAipLuaError;

	/// Convenience method to register a typed handler for this AipFn on a Lua table.
	fn register_typed<H>(lua: &Lua, table: &mlua::Table, handler: H) -> mlua::Result<()>
	where
		H: Fn(Self::Params) -> Result<Self::Response, Self::Error> + 'static,
		Self: Sized,
	{
		register_aip_fn::<Self, H>(lua, table, handler)
	}
}

// endregion: --- Function

// region:    --- AipFn Registration

pub fn register_aip_fn<F: AipFn, H>(lua: &Lua, table: &mlua::Table, handler: H) -> mlua::Result<()>
where
	H: Fn(F::Params) -> Result<F::Response, F::Error> + 'static,
{
	let func = lua.create_function(move |lua, arg: Option<Value>| {
		let Some(arg) = arg else {
			return Err(mlua::Error::RuntimeError(
				"Missing arguments; expected a single params table".to_string(),
			));
		};

		let params = lua_params_from_value::<F::Params>(arg)?;
		match handler(params) {
			Ok(response) => return_success_envelope(lua, response),
			Err(err) => Err(err.into_aip_lua_error()),
		}
	})?;

	table.set(F::NAME, func)?;
	Ok(())
}

// endregion: --- AipFn Registration

// region:    --- Params

pub fn lua_params_from_value<T>(arg: Value) -> mlua::Result<T>
where
	T: DeserializeOwned,
{
	let json_value = match arg {
		Value::Table(table) => {
			let is_empty = table
				.clone()
				.pairs::<Value, Value>()
				.next()
				.transpose()
				.map_err(|err| aip_error("INVALID_PARAMS", Some(err.to_string()), None))?
				.is_none();

			if is_empty {
				serde_json::Value::Object(serde_json::Map::new())
			} else {
				lua_value_to_serde_value(Value::Table(table))
					.map_err(|err| aip_error("INVALID_PARAMS", Some(err.to_string()), None))?
			}
		}
		other => lua_value_to_serde_value(other).map_err(|err| aip_error("INVALID_PARAMS", Some(err.to_string()), None))?,
	};
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
