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
//! - `aip.json.parse(params: { data?: string }) -> { data: any }`
//! - `aip.json.parse_ndjson(params: { data?: string }) -> { data: any[] }`
//! - `aip.json.stringify(params: { data: any }) -> { data: string }`
//! - `aip.json.stringify_pretty(params: { data: any }) -> { data: string }`
//!
//! ---
//!

use crate::Result;
use crate::script::{AipApiError, HandlerRegistry, install_registry_on_table};
use crate::support::jsons;
use mlua::{Lua, Table};
use simple_fs::parse_ndjson_from_reader;
use std::io::BufReader;

// region:    --- Types

// endregion: --- Types

pub fn init_module(lua: &Lua) -> Result<Table> {
	let table = lua.create_table()?;

	let mut registry = HandlerRegistry::new();
	registry
		.append_sync::<_, AipJsonParseParams, AipJsonParseResult, AipApiError>("parse", aip_json_parse_handler)
		.map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
	registry
		.append_sync::<_, AipJsonParseJsonlParams, AipJsonParseJsonlResult, AipApiError>(
			"parse_jsonl",
			aip_json_parse_jsonl_handler,
		)
		.map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
	registry
		.append_sync::<_, AipJsonStringifyParams, AipJsonStringifyResult, AipApiError>(
			"stringify",
			aip_json_stringify_handler,
		)
		.map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
	registry
		.append_sync::<_, AipJsonStringifyParams, AipJsonStringifyPrettyResult, AipApiError>(
			"stringify_pretty",
			aip_json_stringify_pretty_handler,
		)
		.map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;

	install_registry_on_table(lua, &table, registry)?;

	Ok(table)
}

// region:    --- aip.json.parse

/// Parameters for the `parse` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipJsonParseParams {
	/// The JSON string to parse.
	#[serde(default)]
	pub data: Option<String>,
}

/// Result of the `parse` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseResult {
	/// The parsed JSON value.
	pub data: serde_json::Value,
}

fn aip_json_parse_handler(params: AipJsonParseParams) -> core::result::Result<AipJsonParseResult, AipApiError> {
	let Some(content) = params.data else {
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
	pub data: Option<String>,
}

/// Result of the `parse_ndjson` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonParseJsonlResult {
	/// The parsed list of JSON values.
	pub data: Vec<serde_json::Value>,
}

fn aip_json_parse_jsonl_handler(
	params: AipJsonParseJsonlParams,
) -> core::result::Result<AipJsonParseJsonlResult, AipApiError> {
	let Some(content) = params.data else {
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

/// Result of the `stringify` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyResult {
	/// The stringified JSON string.
	pub data: String,
}

fn aip_json_stringify_handler(
	params: AipJsonStringifyParams,
) -> core::result::Result<AipJsonStringifyResult, AipApiError> {
	match serde_json::to_string(&params.data) {
		Ok(str) => Ok(AipJsonStringifyResult { data: str }),
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
	pub data: String,
}

fn aip_json_stringify_pretty_handler(
	params: AipJsonStringifyParams,
) -> core::result::Result<AipJsonStringifyPrettyResult, AipApiError> {
	match serde_json::to_string_pretty(&params.data) {
		Ok(str) => Ok(AipJsonStringifyPrettyResult { data: str }),
		Err(err) => Err(AipApiError {
			code: "STRINGIFY_FAILED".to_string(),
			message: format!("aip.json.stringify_pretty fail to stringify. {err}"),
			details: None,
			cause: None,
		}),
	}
}

// endregion: --- aip.json.stringify_pretty

// region:    --- AipFn marker structs

// endregion: --- AipFn marker structs

// region:    --- Tests

#[cfg(test)]
#[path = "aip_json_tests.rs"]
mod tests;

// endregion: --- Tests
