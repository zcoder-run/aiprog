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
//! - `aip.json.stringify(params: { data: any }) -> { text: string }`
//! - `aip.json.stringify_pretty(params: { data: any }) -> { text: string }`
//!
//! ---
//!

use crate::registry::AipRegistry;
use crate::script::{AipApiError, AipFromLua, AipIntoLua, HandlerRegistry, install_registry_on_table};
use crate::script::{AipApiResult, LuaExt};
use crate::support::jsons;
use crate::{Result, ScriptError, ScriptResult};
use mlua::{Lua, Table};
use simple_fs::parse_ndjson_from_reader;
use std::io::BufReader;

pub fn register(registry: &mut AipRegistry) -> crate::Result<()> {
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

/// Result of the `parse` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseResult {
	/// The parsed JSON value.
	pub data: serde_json::Value,
}

impl AipIntoLua for AipJsonParseResult {
	fn into_lua(self, lua: &Lua) -> ScriptResult<mlua::Value> {
		let table = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		let data_lua = self.data.into_lua(lua)?;
		table.set("data", data_lua).map_err(|e| ScriptError::custom(e.to_string()))?;
		Ok(mlua::Value::Table(table))
	}
}

impl crate::script::AipResponse for AipJsonParseResult {}

fn aip_json_parse_handler(params: AipJsonParseParams) -> AipApiResult<AipJsonParseResult> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseResult {
			data: serde_json::Value::Null,
		});
	};

	match jsons::parse_jsonc_to_serde_value(&content) {
		Ok(Some(json_val)) => Ok(AipJsonParseResult { data: json_val }),
		Ok(None) => Ok(AipJsonParseResult {
			data: serde_json::Value::Null,
		}),
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

/// Result of the `parse_ndjson` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseJsonlResult {
	/// The parsed list of JSON values.
	pub data: Vec<serde_json::Value>,
}

impl AipIntoLua for AipJsonParseJsonlResult {
	fn into_lua(self, lua: &Lua) -> ScriptResult<mlua::Value> {
		let table = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		let mut data_vec = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		for (i, item) in self.data.into_iter().enumerate() {
			let item_lua = item.into_lua(lua)?;
			data_vec.set(i + 1, item_lua).map_err(|e| ScriptError::custom(e.to_string()))?;
		}
		table.set("data", data_vec).map_err(|e| ScriptError::custom(e.to_string()))?;
		Ok(mlua::Value::Table(table))
	}
}

impl crate::script::AipResponse for AipJsonParseJsonlResult {}

fn aip_json_parse_jsonl_handler(params: AipJsonParseJsonlParams) -> AipApiResult<AipJsonParseJsonlResult> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseJsonlResult { data: vec![] });
	};
	let reader = BufReader::new(content.as_bytes());
	match parse_ndjson_from_reader(reader) {
		Ok(values) => Ok(AipJsonParseJsonlResult { data: values }),
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

/// Result of the `stringify` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyResult {
	/// The stringified JSON string.
	pub text: String,
}

impl AipIntoLua for AipJsonStringifyResult {
	fn into_lua(self, lua: &Lua) -> ScriptResult<mlua::Value> {
		let table = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		let text_lua = self.text.into_lua(lua)?;
		table.set("text", text_lua).map_err(|e| ScriptError::custom(e.to_string()))?;
		Ok(mlua::Value::Table(table))
	}
}

impl crate::script::AipResponse for AipJsonStringifyResult {}

fn aip_json_stringify_handler(params: AipJsonStringifyParams) -> AipApiResult<AipJsonStringifyResult> {
	match serde_json::to_string(&params.data) {
		Ok(str) => Ok(AipJsonStringifyResult { text: str }),
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

/// Result of the `stringify_pretty` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyPrettyResult {
	/// The pretty-stringified JSON string.
	pub text: String,
}

impl AipIntoLua for AipJsonStringifyPrettyResult {
	fn into_lua(self, lua: &Lua) -> ScriptResult<mlua::Value> {
		let table = lua.create_table().map_err(|e| ScriptError::custom(e.to_string()))?;
		let text_lua = self.text.into_lua(lua)?;
		table.set("text", text_lua).map_err(|e| ScriptError::custom(e.to_string()))?;
		Ok(mlua::Value::Table(table))
	}
}

impl crate::script::AipResponse for AipJsonStringifyPrettyResult {}

fn aip_json_stringify_pretty_handler(params: AipJsonStringifyParams) -> AipApiResult<AipJsonStringifyPrettyResult> {
	match serde_json::to_string_pretty(&params.data) {
		Ok(str) => Ok(AipJsonStringifyPrettyResult { text: str }),
		Err(err) => Err(AipApiError {
			code: "STRINGIFY_FAILED".to_string(),
			message: format!("aip.json.stringify_pretty fail to stringify. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.stringify_pretty

// region:    --- aip.json init_registry

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

// endregion: --- aip.json init_registry

// region:    --- Tests

#[cfg(test)]
#[path = "aip_json_tests.rs"]
mod tests;

// endregion: --- Tests
