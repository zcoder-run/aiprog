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
//! - `aip.json.parse(params: { text?: string }) -> { data: any }`
//! - `aip.json.parse_jsonl(params: { text?: string }) -> { data: any[] }`
//! - `aip.json.parse_jsonl(params: { text?: string }) -> any[]`
//! - `aip.json.stringify(params: { data: any }) -> { text: string }`
//! - `aip.json.stringify_pretty(params: { data: any }) -> { text: string }`
//!
//! ---
//!

use crate::LuaExt;
use crate::registry::{HandlerError, HandlerResult};
use crate::support::jsons;
use crate::{AipFromLua, AipIntoLua, AipParams};
use crate::{AipOutput, AipRegistry};
use mlua::Lua;
use simple_fs::parse_ndjson_from_reader as parse_jsonl_from_reader;
use std::io::BufReader;

/// Build and return an [`AipRegistry`] containing all `aip.json` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
pub fn init_registry() -> crate::Result<AipRegistry> {
	let mut registry = AipRegistry::from_empty();
	registry.register_sync("aip.json.parse", aip_json_parse_handler)?;
	registry.register_sync("aip.json.parse_jsonl", aip_json_parse_jsonl_handler)?;
	registry.register_sync("aip.json.stringify", aip_json_stringify_handler)?;
	Ok(registry)
}

// region:    --- aip.json.parse

// Parse a json string into a json/lua object
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipJsonParseParams {
	// The JSON string to parse.
	// Can be strict json, or jsonc, or relaxed json
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or("Expected table")?;
		let text = table.x_get_string("text");
		Ok(AipJsonParseParams { text })
	}
}

impl AipParams for AipJsonParseParams {}

/// Output type for `aip.json.parse_jsonl`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseJsonlOutput(pub Vec<serde_json::Value>);

impl AipOutput for AipJsonParseJsonlOutput {}

impl AipIntoLua for AipJsonParseJsonlOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		let seq = lua.create_table().map_err(|e| crate::Error::custom(e.to_string()))?;
		for (i, item) in self.0.into_iter().enumerate() {
			let item_lua = item.into_lua(lua)?;
			seq.set(i + 1, item_lua).map_err(|e| crate::Error::custom(e.to_string()))?;
		}
		Ok(mlua::Value::Table(seq))
	}
}

/// Output type for `aip.json.parse`.
///
/// The parsed JSON value is returned directly to Lua without a wrapper table.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseOutput(pub serde_json::Value);

impl AipIntoLua for AipJsonParseOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		self.0.into_lua(lua)
	}
}

impl AipOutput for AipJsonParseOutput {}

fn aip_json_parse_handler(params: AipJsonParseParams) -> HandlerResult<AipJsonParseOutput> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseOutput(serde_json::Value::Null));
	};

	match jsons::parse_jsonc_to_serde_value(&content) {
		Ok(Some(json_val)) => Ok(AipJsonParseOutput(json_val)),
		Ok(None) => Ok(AipJsonParseOutput(serde_json::Value::Null)),
		Err(err) => Err(HandlerError::custom(format!("aip.json.parse failed. {err}"))),
	}
}

// endregion: --- aip.json.parse

// region:    --- aip.json.parse_jsonl

/// Parameters for the `parse_jsonl` function.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipJsonParseJsonlParams {
	/// The JSONL string to parse.
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseJsonlParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let text = table.x_get_string("text");
		Ok(AipJsonParseJsonlParams { text })
	}
}

impl AipParams for AipJsonParseJsonlParams {}

fn aip_json_parse_jsonl_handler(params: AipJsonParseJsonlParams) -> HandlerResult<AipJsonParseJsonlOutput> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseJsonlOutput(vec![]));
	};
	let reader = BufReader::new(content.as_bytes());
	match parse_jsonl_from_reader(reader) {
		Ok(values) => Ok(AipJsonParseJsonlOutput(values)),
		Err(err) => Err(HandlerError::custom(format!("aip.json.parse_jsonl failed. {err}"))),
	}
}

// endregion: --- aip.json.parse_jsonl

// region:    --- aip.json.stringify

/// Parameters for the `stringify` and `stringify_pretty` functions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipJsonStringifyParams {
	/// The value to serialize to JSON.
	pub data: serde_json::Value,

	/// Tell to format the json
	pub pretty: Option<bool>,
}

impl AipFromLua for AipJsonStringifyParams {
	fn from_lua(lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;

		let data_val: mlua::Value = table.get("data").map_err(|e| e.to_string())?;
		let data = if data_val.is_nil() || data_val.x_is_null() {
			serde_json::Value::Null
		} else {
			serde_json::Value::from_lua(lua, data_val)?
		};

		let pretty = table.x_get_bool("pretty");

		Ok(AipJsonStringifyParams { data, pretty })
	}
}

impl AipParams for AipJsonStringifyParams {}

/// Output type for `aip.json.stringify`.
///
/// The serialized JSON string is returned directly to Lua as a Lua string,
/// without a wrapper table.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyOutput(pub String);

impl AipIntoLua for AipJsonStringifyOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		self.0.into_lua(lua)
	}
}

impl AipOutput for AipJsonStringifyOutput {}

fn aip_json_stringify_handler(params: AipJsonStringifyParams) -> HandlerResult<AipJsonStringifyOutput> {
	let res = if params.pretty.unwrap_or_default() {
		serde_json::to_string_pretty(&params.data)
	} else {
		serde_json::to_string(&params.data)
	};

	match res {
		Ok(str) => Ok(AipJsonStringifyOutput(str)),
		Err(err) => Err(HandlerError::custom(format!(
			"aip.json.stringify fail to stringify. {err}"
		))),
	}
}

// endregion: --- aip.json.stringify

// region:    --- Tests

#[cfg(test)]
#[path = "aip_json_tests.rs"]
mod tests;

// endregion: --- Tests
