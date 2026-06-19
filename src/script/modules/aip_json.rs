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

use crate::registry::AipRegistry;
use crate::script::{AipApiError, AipFromLua, AipIntoLua};
use crate::script::{AipApiResult, LuaExt};
use crate::support::jsons;
use crate::{ScriptError, ScriptResult};
use mlua::Lua;
use simple_fs::parse_ndjson_from_reader;
use std::io::BufReader;

use aiprog_macros::AipIntoLua;
use aiprog_macros::AipResponse;

// region:    --- Response Types (new single-value returns)

/// Response type for `aip.json.parse`.
///
/// The parsed JSON value is returned directly to Lua without a wrapper table.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipResponse)]
pub struct AipJsonParseResponse(pub serde_json::Value);

/// Response type for `aip.json.stringify`.
///
/// The serialized JSON string is returned directly to Lua as a Lua string,
/// without a wrapper table.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipResponse)]
pub struct AipJsonStringifyResponse(pub String);

/// Response type for `aip.json.stringify_pretty`.
///
/// The pretty-printed JSON string is returned directly to Lua as a Lua string,
/// without a wrapper table.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipResponse)]
pub struct AipJsonStringifyPrettyResponse(pub String);

/// Response type for `aip.json.parse_jsonl`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipResponse)]
pub struct AipJsonParseJsonlResponse(pub Vec<serde_json::Value>);

impl AipIntoLua for AipJsonParseJsonlResponse {
	fn into_lua(self, lua: &Lua) -> ScriptResult<mlua::Value> {
		let seq = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		for (i, item) in self.0.into_iter().enumerate() {
			let item_lua = item.into_lua(lua)?;
			seq.set(i + 1, item_lua).map_err(|e| ScriptError::custom(e.to_string()))?;
		}
		Ok(mlua::Value::Table(seq))
	}
}

// endregion: --- Response Types

/// Build and return an [`AipRegistry`] containing all `aip.json` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
pub fn init_registry() -> crate::Result<AipRegistry> {
	let mut registry = AipRegistry::default();
	register(&mut registry)?;
	Ok(registry)
}

fn register(registry: &mut AipRegistry) -> crate::Result<()> {
	registry.register_sync::<_, _, _, _>("aip.json.parse", aip_json_parse_handler)?;
	registry.register_sync::<_, _, _, _>("aip.json.parse_jsonl", aip_json_parse_jsonl_handler)?;
	registry.register_sync::<_, _, _, _>("aip.json.stringify", aip_json_stringify_handler)?;
	registry.register_sync::<_, _, _, _>("aip.json.stringify_pretty", aip_json_stringify_pretty_handler)?;
	Ok(())
}

// region:    --- aip.json.parse

/// Parameters for the `parse` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonParseParams {
	/// The JSON string to parse.
	#[serde(default)]
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseParams {
	fn from_lua(lua: &Lua, value: mlua::Value) -> ScriptResult<Self> {
		let table = value.as_table().ok_or("Expected table")?;
		let text_val: mlua::Value = table.get("text")?;
		let text = Option::<String>::from_lua(lua, text_val)?;
		Ok(AipJsonParseParams { text })
	}
}

impl crate::script::AipParams for AipJsonParseParams {}

fn aip_json_parse_handler(params: AipJsonParseParams) -> AipApiResult<AipJsonParseResponse> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseResponse(serde_json::Value::Null));
	};

	match jsons::parse_jsonc_to_serde_value(&content) {
		Ok(Some(json_val)) => Ok(AipJsonParseResponse(json_val)),
		Ok(None) => Ok(AipJsonParseResponse(serde_json::Value::Null)),
		Err(err) => Err(AipApiError {
			code: "PARSE_FAILED".to_string(),
			message: format!("aip.json.parse failed. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.parse

// region:    --- aip.json.parse_ndjson

/// Parameters for the `parse_ndjson` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonParseJsonlParams {
	/// The NDJSON string to parse.
	#[serde(default)]
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseJsonlParams {
	fn from_lua(lua: &Lua, value: mlua::Value) -> ScriptResult<Self> {
		let table = value.as_table().ok_or_else(|| ScriptError::custom("Expected table"))?;
		let text_val: mlua::Value = table.get("text")?;
		let text = Option::<String>::from_lua(lua, text_val)?;
		Ok(AipJsonParseJsonlParams { text })
	}
}

impl crate::script::AipParams for AipJsonParseJsonlParams {}

fn aip_json_parse_jsonl_handler(params: AipJsonParseJsonlParams) -> AipApiResult<AipJsonParseJsonlResponse> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseJsonlResponse(vec![]));
	};
	let reader = BufReader::new(content.as_bytes());
	match parse_ndjson_from_reader(reader) {
		Ok(values) => Ok(AipJsonParseJsonlResponse(values)),
		Err(err) => Err(AipApiError {
			code: "PARSE_FAILED".to_string(),
			message: format!("aip.json.parse_jsonl failed. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.parse_ndjson

// region:    --- aip.json.stringify

/// Parameters for the `stringify` and `stringify_pretty` functions.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonStringifyParams {
	/// The value to serialize to JSON.
	pub data: serde_json::Value,
}

impl AipFromLua for AipJsonStringifyParams {
	fn from_lua(lua: &Lua, value: mlua::Value) -> ScriptResult<Self> {
		let table = value.as_table().ok_or_else(|| ScriptError::custom("Expected table"))?;
		let data_val: mlua::Value = table.get("data").map_err(|e| ScriptError::custom(e.to_string()))?;
		let data = if data_val.is_nil() || data_val.x_is_null() {
			serde_json::Value::Null
		} else {
			serde_json::Value::from_lua(lua, data_val)?
		};
		Ok(AipJsonStringifyParams { data })
	}
}

impl crate::script::AipParams for AipJsonStringifyParams {}

fn aip_json_stringify_handler(params: AipJsonStringifyParams) -> AipApiResult<AipJsonStringifyResponse> {
	match serde_json::to_string(&params.data) {
		Ok(str) => Ok(AipJsonStringifyResponse(str)),
		Err(err) => Err(AipApiError {
			code: "STRINGIFY_FAILED".to_string(),
			message: format!("aip.json.stringify fail to stringify. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.stringify

// region:    --- aip.json.stringify_pretty

fn aip_json_stringify_pretty_handler(params: AipJsonStringifyParams) -> AipApiResult<AipJsonStringifyPrettyResponse> {
	match serde_json::to_string_pretty(&params.data) {
		Ok(str) => Ok(AipJsonStringifyPrettyResponse(str)),
		Err(err) => Err(AipApiError {
			code: "STRINGIFY_FAILED".to_string(),
			message: format!("aip.json.stringify_pretty fail to stringify. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.stringify_pretty

// region:    --- Tests

#[cfg(test)]
#[path = "aip_json_tests.rs"]
mod tests;

// endregion: --- Tests
