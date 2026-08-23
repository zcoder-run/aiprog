//! Defines the `text` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.text` module exposes text formatting and manipulation utilities.
//!
//! ### Functions
//!
//! - `aip.text.format_size(params: { size?: integer, lowest_unit?: string, trim?: boolean }) -> string | nil`
//!
//! ---

#![allow(non_camel_case_types)]

use crate::base::text::format_pretty_size;
use crate::registry::HandlerResult;
use crate::{AipFromLua, AipIntoLua, AipParams, HandlerCallContext};
use crate::{AipModule, LuaExt};
use crate::{AipOutput, AipRegistryBuilder};
use aiprog_macros::aip_handler;
use aiprog_macros::register_handler;
use mlua::Lua;
use simple_fs::PrettySizeOptions;

#[derive(Debug, Clone, Copy, Default)]
pub struct TextModule;

impl AipModule for TextModule {
	fn register(mut builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		register_handler!(builder, "aip.text.format_size", aip_text_format_size_handler)?;
		Ok(builder)
	}
}

// region:    --- aip.text.format_size

/// Parameters for `aip.text.format_size`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipTextFormatSizeParams {
	/// The size in bytes (integer or number). If nil/absent, returns nil.
	pub size: Option<u64>,

	/// The lowest unit to display (e.g. "B", "KB", "MB", "GB", "TB").
	pub lowest_unit: Option<String>,

	/// Whether to trim whitespace from the result (default: false).
	pub trim: Option<bool>,
}

impl AipFromLua for AipTextFormatSizeParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Params expected to be a table, but was of type '{}'",
				value.type_name()
			))
		})?;

		let size = match table.x_try_get_value("size")? {
			Some(mlua::Value::Integer(i)) => Some(i.max(0) as u64),
			Some(mlua::Value::Number(n)) => Some(n.round().max(0.0) as u64),
			Some(mlua::Value::Nil) | None => None,
			Some(other) => {
				return Err(crate::Error::custom(format!(
					"Property 'size' expected to be a number, but was '{}'",
					other.type_name()
				)));
			}
		};

		let lowest_unit = table.x_try_get_string("lowest_unit")?;
		let trim = table.x_try_get_bool("trim")?;

		Ok(AipTextFormatSizeParams {
			size,
			lowest_unit,
			trim,
		})
	}
}

impl AipParams for AipTextFormatSizeParams {}

/// Output type for `aip.text.format_size`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipTextFormatSizeOutput(pub Option<String>);

impl AipIntoLua for AipTextFormatSizeOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		match self.0 {
			Some(s) => s.into_lua(lua),
			None => Ok(mlua::Value::Nil),
		}
	}
}

impl AipOutput for AipTextFormatSizeOutput {}

/// Formats a byte size into a human-readable 9 characters string.
#[aip_handler]
fn aip_text_format_size_handler(
	_call: HandlerCallContext,
	params: AipTextFormatSizeParams,
) -> HandlerResult<AipTextFormatSizeOutput> {
	let Some(size) = params.size else {
		return Ok(AipTextFormatSizeOutput(None));
	};

	let pretty_options = params.lowest_unit.as_deref().map(PrettySizeOptions::from);
	let pretty = format_pretty_size(size, pretty_options);
	let result = if params.trim.unwrap_or_default() {
		pretty.trim().to_string()
	} else {
		pretty
	};

	Ok(AipTextFormatSizeOutput(Some(result)))
}

// endregion: --- aip.text.format_size

// region:    --- Tests

#[cfg(test)]
#[path = "aip_text_tests.rs"]
mod tests;

// endregion: --- Tests
