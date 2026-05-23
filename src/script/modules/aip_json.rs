//! Defines the `json` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.json` module exposes functions to parse and stringify JSON content.
//!
//! IMPORTANT: By default, this support the parsing of jsonc content, meaning json with optional comments
//!
//! - Parse function will return nil if content is nil
//! - stringify return a single line
//! - use `stringify_pretty` for idented multi-line
//!
//! ### Functions
//!
//! - `aip.json.parse(content: string | nil) -> table | nil`
//! - `aip.json.parse_ndjson(content: string | nil) -> table[] | nil`
//! - `aip.json.stringify(content: table) -> string`
//! - `aip.json.stringify_pretty(content: table) -> string`
//!
//! ---
//!

use crate::script::helpers::{LuaValueExt as _, lua_value_to_serde_value, serde_value_to_lua_value};
use crate::support::jsons;
use crate::{Error, Result};
use mlua::{Lua, Table, Value};
use simple_fs::parse_ndjson_from_reader;
use std::io::BufReader;

// region:    --- Types

// -- Internal API Envelope

#[derive(serde::Serialize)]
struct ApiResponse<T> {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<ApiSuccess<T>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<ApiError>,
}

#[derive(serde::Serialize)]
struct ApiSuccess<T> {
	pub data: T,
}

#[derive(serde::Serialize)]
struct ApiError {
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<ApiErrorData>,
}

#[derive(serde::Serialize)]
struct ApiErrorData {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub full_message: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cause: Option<String>,
}

impl<T> ApiResponse<T> {
	pub fn success(data: T) -> Self {
		Self {
			result: Some(ApiSuccess { data }),
			error: None,
		}
	}

	pub fn error(message: impl Into<String>, full_message: Option<String>, cause: Option<String>) -> Self {
		Self {
			result: None,
			error: Some(ApiError {
				message: message.into(),
				data: Some(ApiErrorData { full_message, cause }),
			}),
		}
	}
}

fn return_success_envelope<T: serde::Serialize>(lua: &Lua, data: T) -> mlua::Result<Value> {
	let response = ApiResponse::success(data);
	let serde_val = serde_json::to_value(&response)
		.map_err(|err| mlua::Error::RuntimeError(format!("Failed to serialize API success response: {err}")))?;
	serde_value_to_lua_value(lua, serde_val).map_err(Into::into)
}

fn return_error_envelope(
	lua: &Lua,
	message: &str,
	full_message: Option<String>,
	cause: Option<String>,
) -> mlua::Result<Value> {
	let response: ApiResponse<serde_json::Value> = ApiResponse::error(message, full_message, cause);
	let serde_val = serde_json::to_value(&response)
		.map_err(|err| mlua::Error::RuntimeError(format!("Failed to serialize API error response: {err}")))?;
	serde_value_to_lua_value(lua, serde_val).map_err(Into::into)
}

// endregion: --- Types

pub fn init_module(lua: &Lua) -> Result<Table> {
	let table = lua.create_table()?;

	let parse_fn = lua.create_function(move |lua, arg: Option<Value>| aip_json_parse(lua, arg))?;
	let parse_ndjson_fn = lua.create_function(move |lua, arg: Option<Value>| parse_ndjson(lua, arg))?;
	let stringify_fn = lua.create_function(move |lua, arg: Option<Value>| stringify(lua, arg))?;
	let stringify_pretty_fn = lua.create_function(move |lua, arg: Option<Value>| stringify_pretty(lua, arg))?;
	// stringify_to_line is now an alias for stringify
	let stringify_to_line_fn = stringify_fn.clone();

	table.set("parse", parse_fn)?;
	table.set("parse_ndjson", parse_ndjson_fn)?;
	table.set("stringify", stringify_fn)?;
	table.set("stringify_pretty", stringify_pretty_fn)?;

	// deprecated, should use stringify
	table.set("stringify_to_line", stringify_to_line_fn)?;

	Ok(table)
}

// region:    --- aip.json.parse

/// Parameters for the `parse` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonParseParams {
	/// The JSON string to parse.
	#[serde(alias = "data")]
	pub content: Option<String>,
}

/// Result of the `parse` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct AipJsonParseResult {
	/// The parsed JSON value.
	pub value: serde_json::Value,
}

fn aip_json_parse(lua: &Lua, arg: Option<Value>) -> mlua::Result<Value> {
	let Some(arg) = arg else {
		return Ok(Value::Nil);
	};

	let is_new_api = if let Value::Table(ref t) = arg {
		t.x_get_value("content").is_some() || t.x_get_value("data").is_some()
	} else {
		false
	};

	if is_new_api {
		let json_value = match lua_value_to_serde_value(arg) {
			Ok(v) => v,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let params: AipJsonParseParams = match serde_json::from_value(json_value) {
			Ok(p) => p,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let Some(content) = params.content else {
			return return_success_envelope(lua, serde_json::Value::Null);
		};
		match jsons::parse_jsonc_to_serde_value(&content) {
			Ok(Some(json_val)) => return_success_envelope(lua, json_val),
			Ok(None) => return_success_envelope(lua, serde_json::Value::Null),
			Err(err) => return_error_envelope(lua, "PARSE_FAILED", Some(format!("aip.json.parse failed. {err}")), None),
		}
	} else {
		let content = match arg {
			Value::String(s) => Some(s.to_str()?.to_string()),
			Value::Nil => None,
			_ => arg.x_to_string(),
		};
		let Some(content) = content else {
			return Ok(Value::Nil);
		};
		let json_value = match jsons::parse_jsonc_to_serde_value(&content) {
			Ok(val) => val,
			Err(err) => return Err(Error::custom(format!("aip.json.parse failed. {err}")).into()),
		};
		let json_value = json_value.unwrap_or_default();
		let lua_value = serde_value_to_lua_value(lua, json_value)?;
		Ok(lua_value)
	}
}

// endregion: --- aip.json.parse

// region:    --- aip.json.parse_ndjson

/// Parameters for the `parse_ndjson` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonParseNdjsonParams {
	/// The NDJSON string to parse.
	#[serde(alias = "data")]
	pub content: Option<String>,
}

/// Result of the `parse_ndjson` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct AipJsonParseNdjsonResult {
	/// The parsed list of JSON values.
	pub values: Vec<serde_json::Value>,
}

fn parse_ndjson(lua: &Lua, arg: Option<Value>) -> mlua::Result<Value> {
	let Some(arg) = arg else {
		return Ok(Value::Nil);
	};

	let is_new_api = if let Value::Table(ref t) = arg {
		t.x_get_value("content").is_some() || t.x_get_value("data").is_some()
	} else {
		false
	};

	if is_new_api {
		let json_value = match lua_value_to_serde_value(arg) {
			Ok(v) => v,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let params: AipJsonParseNdjsonParams = match serde_json::from_value(json_value) {
			Ok(p) => p,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let Some(content) = params.content else {
			return return_success_envelope(lua, serde_json::Value::Array(vec![]));
		};
		let reader = BufReader::new(content.as_bytes());
		match parse_ndjson_from_reader(reader) {
			Ok(values) => {
				let result = AipJsonParseNdjsonResult { values };
				return_success_envelope(lua, result)
			}
			Err(err) => return_error_envelope(
				lua,
				"PARSE_FAILED",
				Some(format!("aip.json.parse_ndjson failed. {err}")),
				None,
			),
		}
	} else {
		let content = match arg {
			Value::String(s) => Some(s.to_str()?.to_string()),
			Value::Nil => None,
			_ => arg.x_to_string(),
		};
		let Some(content) = content else {
			return Ok(Value::Nil);
		};
		let reader = BufReader::new(content.as_bytes());
		match parse_ndjson_from_reader(reader) {
			Ok(values) => {
				let values = serde_json::Value::Array(values);
				let lua_value = serde_value_to_lua_value(lua, values)?;
				Ok(lua_value)
			}
			Err(err) => Err(Error::custom(format!("aip.json.parse_ndjson failed. {err}")).into()),
		}
	}
}

// endregion: --- aip.json.parse_ndjson

// region:    --- aip.json.stringify

/// Parameters for the `stringify` and `stringify_pretty` functions.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonStringifyParams {
	/// The value to serialize to JSON.
	#[serde(alias = "data")]
	pub value: serde_json::Value,
}

/// Result of the `stringify` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct AipJsonStringifyResult {
	/// The stringified JSON string.
	pub content: String,
}

/// ## Lua Documentation
/// ---
/// Stringify a table into a single line JSON string.
///
/// Good for newline json or compact representation.
///
/// ```lua
/// -- API Signature
/// aip.json.stringify(content: table): string
/// ```
///
/// Convert a table into a single line JSON string.
///
/// ### Arguments
///
/// - `content: table` - The Lua table to stringify.
///
/// ### Returns
///
/// - `string` - A string containing the JSON representation of the input table,
///   without any indentation or extra whitespace (except within string values).
///
/// ### Example
///
/// ```lua
/// local obj = {
///     name = "John",
///     age = 30
/// }
/// local json_str = aip.json.stringify(obj)
/// -- Result will be:
/// -- {"name":"John","age":30}
/// ```
///
/// ### Error
///
/// Returns an error if the input Lua value cannot be serialized into JSON.
///
/// ```ts
/// {
///   error: string  // Error message from JSON stringification, e.g., "aip.json.stringify fail to stringify. ..."
/// }
/// ```
fn stringify(lua: &Lua, arg: Option<Value>) -> mlua::Result<Value> {
	let Some(arg) = arg else {
		let json_value = serde_json::Value::Null;
		let str = serde_json::to_string(&json_value)
			.map_err(|err| Error::custom(format!("aip.json.stringify fail to stringify. {err}")))?;
		return Ok(Value::String(lua.create_string(&str)?));
	};

	let is_new_api = if let Value::Table(ref t) = arg {
		let mut has_value_or_data = false;
		let mut key_count = 0;
		for pair in t.clone().pairs::<Value, Value>() {
			let (k, _) = pair?;
			key_count += 1;
			if let Some(k_str) = k.x_to_string()
				&& (k_str == "value" || k_str == "data")
			{
				has_value_or_data = true;
			}
		}
		key_count == 1 && has_value_or_data
	} else {
		false
	};

	if is_new_api {
		let json_value = match lua_value_to_serde_value(arg) {
			Ok(v) => v,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let params: AipJsonStringifyParams = match serde_json::from_value(json_value) {
			Ok(p) => p,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		match serde_json::to_string(&params.value) {
			Ok(str) => return_success_envelope(lua, AipJsonStringifyResult { content: str }),
			Err(err) => return_error_envelope(
				lua,
				"STRINGIFY_FAILED",
				Some(format!("aip.json.stringify fail to stringify. {err}")),
				None,
			),
		}
	} else {
		let json_value = lua_value_to_serde_value(arg)?;
		match serde_json::to_string(&json_value) {
			Ok(str) => Ok(Value::String(lua.create_string(&str)?)),
			Err(err) => Err(Error::custom(format!("aip.json.stringify fail to stringify. {err}")).into()),
		}
	}
}

// endregion: --- aip.json.stringify

// region:    --- aip.json.stringify_pretty

/// Result of the `stringify_pretty` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct AipJsonStringifyPrettyResult {
	/// The pretty-stringified JSON string.
	pub content: String,
}

/// ## Lua Documentation
/// ---
/// Stringify a table into a JSON string with pretty formatting.
///
/// ```lua
/// -- API Signature
/// aip.json.stringify_pretty(content: table): string
/// ```
///
/// Convert a table into a JSON string with pretty formatting using 2 spaces indentation.
///
/// ### Arguments
///
/// - `content: table` - The Lua table to stringify.
///
/// ### Returns
///
/// - `string` - A string containing the pretty-formatted JSON representation
///   of the input table, using newlines and 2-space indentation.
///
/// ### Example
///
/// ```lua
/// local obj = {
///     name = "John",
///     age = 30
/// }
/// local json_str = aip.json.stringify_pretty(obj)
/// -- Result will be similar to:
/// -- {
/// --   "name": "John",
/// --   "age": 30
/// -- }
/// ```
///
/// ### Error
///
/// Returns an error if the input Lua value cannot be serialized into JSON.
///
/// ```ts
/// {
///   error: string  // Error message from JSON stringification, e.g., "aip.json.stringify_pretty fail to stringify. ..."
/// }
/// ```
fn stringify_pretty(lua: &Lua, arg: Option<Value>) -> mlua::Result<Value> {
	let Some(arg) = arg else {
		let json_value = serde_json::Value::Null;
		let str = serde_json::to_string_pretty(&json_value)
			.map_err(|err| Error::custom(format!("aip.json.stringify_pretty fail to stringify. {err}")))?;
		return Ok(Value::String(lua.create_string(&str)?));
	};

	let is_new_api = if let Value::Table(ref t) = arg {
		let mut has_value_or_data = false;
		let mut key_count = 0;
		for pair in t.clone().pairs::<Value, Value>() {
			let (k, _) = pair?;
			key_count += 1;
			if let Some(k_str) = k.x_to_string()
				&& (k_str == "value" || k_str == "data")
			{
				has_value_or_data = true;
			}
		}
		key_count == 1 && has_value_or_data
	} else {
		false
	};

	if is_new_api {
		let json_value = match lua_value_to_serde_value(arg) {
			Ok(v) => v,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		let params: AipJsonStringifyParams = match serde_json::from_value(json_value) {
			Ok(p) => p,
			Err(err) => return return_error_envelope(lua, "INVALID_PARAMS", Some(err.to_string()), None),
		};
		match serde_json::to_string_pretty(&params.value) {
			Ok(str) => return_success_envelope(lua, AipJsonStringifyPrettyResult { content: str }),
			Err(err) => return_error_envelope(
				lua,
				"STRINGIFY_FAILED",
				Some(format!("aip.json.stringify_pretty fail to stringify. {err}")),
				None,
			),
		}
	} else {
		let json_value = lua_value_to_serde_value(arg)?;
		match serde_json::to_string_pretty(&json_value) {
			Ok(str) => Ok(Value::String(lua.create_string(&str)?)),
			Err(err) => Err(Error::custom(format!("aip.json.stringify_pretty fail to stringify. {err}")).into()),
		}
	}
}

// endregion: --- aip.json.stringify_pretty

// region:    --- Tests

#[cfg(test)]
#[path = "aip_json_tests.rs"]
mod tests;

// endregion: --- Tests
