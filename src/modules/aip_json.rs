//! Defines the `json` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.json` module exposes functions to parse and stringify JSON content.
//!
//! IMPORTANT: By default, this supports the parsing of JSONC content, meaning JSON with optional comments.
//!
//! - Parse functions will return nil if content is nil or absent.
//! - stringify will return nil if data is nil or absent.
//! - stringify returns a single-line JSON string; use `pretty = true` for indented multi-line output.
//!
//! ### Functions
//!
//! - `aip.json.parse(params: { text?: string }) -> any`
//! - `aip.json.parse_jsonl(params: { text?: string }) -> any[]`
//! - `aip.json.stringify(params: { data: any, pretty?: boolean }) -> string | nil`
//!
//! ---
//!

#![allow(non_camel_case_types)]

use crate::LuaExt;
use crate::registry::{HandlerError, HandlerResult};
use crate::support::jsons;
use crate::{AipFromLua, AipIntoLua, AipParams};
use crate::{AipOutput, AipRegistry};
use aiprog_macros::aip_handler;
use aiprog_macros::register_handler;
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
	register_handler!(registry, "aip.json.parse", aip_json_parse_handler)?;
	register_handler!(registry, "aip.json.parse_jsonl", aip_json_parse_jsonl_handler)?;
	register_handler!(registry, "aip.json.stringify", aip_json_stringify_handler)?;
	Ok(registry)
}

// region:    --- aip.json.parse

// Parse a json string into a json/lua object
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipJsonParseParams {
	// The JSON string to parse.
	// Can be strict json, or jsonc, or relaxed json.
	// If nil/null will return null
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Params expected to be a table, but was of type '{}'",
				value.type_name()
			))
		})?;
		let text = table.x_try_get_string("text")?;
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
/// When the text is nil/absent, returns Lua nil.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseOutput(pub Option<serde_json::Value>);

impl AipIntoLua for AipJsonParseOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		match self.0 {
			Some(v) => v.into_lua(lua),
			None => Ok(mlua::Value::Nil),
		}
	}
}

impl AipOutput for AipJsonParseOutput {}

/// Parses a JSON string into a Lua value.
#[aip_handler]
fn aip_json_parse_handler(params: AipJsonParseParams) -> HandlerResult<AipJsonParseOutput> {
	let Some(content) = params.text else {
		return Ok(AipJsonParseOutput(None));
	};

	match jsons::parse_jsonc_to_serde_value(&content) {
		Ok(Some(json_val)) => Ok(AipJsonParseOutput(Some(json_val))),
		Ok(None) => Ok(AipJsonParseOutput(None)),
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
	/// If nil/null will return nil
	pub text: Option<String>,
}

impl AipFromLua for AipJsonParseJsonlParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Params expected to be a table, but was of type '{}'",
				value.type_name()
			))
		})?;
		let text = table.x_try_get_string("text")?;
		Ok(AipJsonParseJsonlParams { text })
	}
}

impl AipParams for AipJsonParseJsonlParams {}

/// Parses a JSONL content (json lines) into a list of JSON values (one per line).
///
#[aip_handler]
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

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipJsonStringifyParams {
	/// The value to serialize to JSON.
	/// If nil/null, will return nil
	pub data: serde_json::Value,

	/// Tell to format the json with indent (default false, i.e. single line)
	pub pretty: Option<bool>,
}

impl AipFromLua for AipJsonStringifyParams {
	fn from_lua(lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Params expected to be a table, but was of type '{}'",
				value.type_name()
			))
		})?;

		let data = match table.x_try_get_value("data")? {
			Some(data_val) if !data_val.x_is_null() => serde_json::Value::from_lua(lua, data_val)?,
			_ => serde_json::Value::Null,
		};

		let pretty = table.x_try_get_bool("pretty")?;

		Ok(AipJsonStringifyParams { data, pretty })
	}
}

impl AipParams for AipJsonStringifyParams {}

/// Output type for `aip.json.stringify`.
///
/// The serialized JSON string is returned directly to Lua as a Lua string,
/// without a wrapper table.
/// When the data is nil/null/absent, the output is Lua `nil`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyOutput(pub Option<String>);

impl AipIntoLua for AipJsonStringifyOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		match self.0 {
			Some(s) => s.into_lua(lua),
			None => Ok(mlua::Value::Nil),
		}
	}
}

impl AipOutput for AipJsonStringifyOutput {}

/// Serializes a Lua value to a JSON string.
/// Returns Lua `nil` when the data is nil/null/absent.
#[aip_handler]
fn aip_json_stringify_handler(params: AipJsonStringifyParams) -> HandlerResult<AipJsonStringifyOutput> {
	if params.data.is_null() {
		return Ok(AipJsonStringifyOutput(None));
	}

	let res = if params.pretty.unwrap_or_default() {
		serde_json::to_string_pretty(&params.data)
	} else {
		serde_json::to_string(&params.data)
	};

	match res {
		Ok(str) => Ok(AipJsonStringifyOutput(Some(str))),
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
