//! Defines the `md` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.md` module provides functions for generating and formatting Markdown content.
//!
//! ### Functions
//!
//! - `aip.md.make_table(params: { headers?: string[], rows: (string | number | boolean | null)[][] }) -> string`
//!
//! ---
//!

use crate::LuaExt;
use crate::base::md::{CellValue, make_table};
use crate::registry::HandlerResult;
use crate::{AipFromLua, AipIntoLua, AipOutput, AipParams, HandlerCallContext};
use crate::{AipRegistry, AipRegistryBuilder};
use aiprog_macros::{aip_handler, register_handler};
use mlua::Lua;

/// Build and return an [`AipRegistry`] containing all `aip.md` handlers.
#[allow(dead_code)]
pub fn init_registry() -> crate::Result<AipRegistry> {
	Ok(register(AipRegistryBuilder::default())?.build())
}

pub fn register(mut registry: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
	register_handler!(registry, "aip.md.make_table", aip_md_make_table_handler)?;
	Ok(registry)
}

// region:    --- aip.md.make_table

/// Parameters for `aip.md.make_table`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipMdMakeTableParams {
	/// Optional list of column header titles.
	///
	/// Recommended: Always provide `headers` whenever column names are available or known, to produce standard Markdown pipe tables with header and delimiter rows.
	pub headers: Option<Vec<String>>,

	/// 2D array of table rows. Each cell can be a string, number, boolean, or null/nil.
	pub rows: Vec<Vec<CellValue>>,
}

fn parse_cell_value(val: mlua::Value) -> crate::Result<CellValue> {
	match val {
		mlua::Value::Nil => Ok(CellValue::Null),
		mlua::Value::Boolean(b) => Ok(CellValue::Bool(b)),
		mlua::Value::Integer(i) => Ok(CellValue::Number(i as f64)),
		mlua::Value::Number(n) => Ok(CellValue::Number(n)),
		mlua::Value::String(s) => Ok(CellValue::String(s.to_str()?.to_string())),
		mlua::Value::LightUserData(_) if val.x_is_null() => Ok(CellValue::Null),
		other if other.x_is_null() => Ok(CellValue::Null),
		other => Err(crate::Error::custom(format!(
			"Unsupported cell type '{}'",
			other.type_name()
		))),
	}
}

fn parse_row(val: mlua::Value) -> crate::Result<Vec<CellValue>> {
	let table = val.as_table().ok_or_else(|| {
		crate::Error::custom(format!(
			"Row expected to be a table, but was of type '{}'",
			val.type_name()
		))
	})?;

	let mut max_idx: usize = 0;
	let mut entries: Vec<(usize, CellValue)> = Vec::new();

	for pair in table.pairs::<mlua::Value, mlua::Value>() {
		let (k, v) = pair?;
		let idx = match k {
			mlua::Value::Integer(i) if i > 0 => i as usize,
			mlua::Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => n as usize,
			_ => {
				return Err(crate::Error::custom(format!(
					"Invalid row key type '{}', expected positive integer index",
					k.type_name()
				)));
			}
		};
		if idx > max_idx {
			max_idx = idx;
		}
		let cell = parse_cell_value(v)?;
		entries.push((idx, cell));
	}

	let mut row = vec![CellValue::Null; max_idx];
	for (idx, cell) in entries {
		if idx > 0 && idx <= max_idx {
			row[idx - 1] = cell;
		}
	}
	Ok(row)
}

impl AipFromLua for AipMdMakeTableParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Params expected to be a table, but was of type '{}'",
				value.type_name()
			))
		})?;

		let headers = if let Some(hdrs_val) = table.x_try_get_value("headers")? {
			if hdrs_val.is_nil() || hdrs_val.x_is_null() {
				None
			} else if let Some(hdrs_table) = hdrs_val.as_table() {
				let mut max_idx: usize = 0;
				let mut entries: Vec<(usize, String)> = Vec::new();
				for pair in hdrs_table.pairs::<mlua::Value, mlua::Value>() {
					let (k, v) = pair?;
					let idx = match k {
						mlua::Value::Integer(i) if i > 0 => i as usize,
						mlua::Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => n as usize,
						_ => {
							return Err(crate::Error::custom(
								"Invalid header key type, expected positive integer index",
							));
						}
					};
					if idx > max_idx {
						max_idx = idx;
					}
					let s = match v {
						mlua::Value::String(s) => s.to_str()?.to_string(),
						other => {
							return Err(crate::Error::custom(format!(
								"Header item must be a string, got '{}'",
								other.type_name()
							)));
						}
					};
					entries.push((idx, s));
				}
				let mut hdrs = vec![String::new(); max_idx];
				for (idx, s) in entries {
					if idx > 0 && idx <= max_idx {
						hdrs[idx - 1] = s;
					}
				}
				Some(hdrs)
			} else {
				return Err(crate::Error::custom(
					"Property 'headers' must be a table (array of strings)",
				));
			}
		} else {
			None
		};

		let rows_val = table
			.x_try_get_value("rows")?
			.ok_or_else(|| crate::Error::custom("Missing required property 'rows'"))?;

		let rows_table = rows_val.as_table().ok_or_else(|| {
			crate::Error::custom(format!(
				"Property 'rows' must be a table, got '{}'",
				rows_val.type_name()
			))
		})?;

		let mut max_row_idx: usize = 0;
		let mut row_entries: Vec<(usize, Vec<CellValue>)> = Vec::new();
		for pair in rows_table.pairs::<mlua::Value, mlua::Value>() {
			let (k, v) = pair?;
			let idx = match k {
				mlua::Value::Integer(i) if i > 0 => i as usize,
				mlua::Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => n as usize,
				_ => {
					return Err(crate::Error::custom(
						"Invalid rows key type, expected positive integer index",
					));
				}
			};
			if idx > max_row_idx {
				max_row_idx = idx;
			}
			let row = parse_row(v)?;
			row_entries.push((idx, row));
		}

		let mut rows = vec![Vec::new(); max_row_idx];
		for (idx, row) in row_entries {
			if idx > 0 && idx <= max_row_idx {
				rows[idx - 1] = row;
			}
		}

		Ok(AipMdMakeTableParams { headers, rows })
	}
}

impl AipParams for AipMdMakeTableParams {}

/// Output type for `aip.md.make_table`.
///
/// The Markdown table string is returned directly to Lua as a string.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipMdMakeTableOutput(pub String);

impl AipIntoLua for AipMdMakeTableOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		self.0.into_lua(lua)
	}
}

impl AipOutput for AipMdMakeTableOutput {}

/// Formats structured headers and row data into a Markdown pipe table.
///
/// ### When to use:
///  Use this function when building a markdown table programmatically from a dataset.
///  Always provide `headers` whenever column names are available or known,
///  to ensure the output includes a proper header row and separator line.
#[aip_handler]
fn aip_md_make_table_handler(
	_call: HandlerCallContext,
	params: AipMdMakeTableParams,
) -> HandlerResult<AipMdMakeTableOutput> {
	let headers_slice = params.headers.as_deref();
	let table_str = make_table(headers_slice, &params.rows);
	Ok(AipMdMakeTableOutput(table_str))
}

// endregion: --- aip.md.make_table

// region:    --- Tests

#[cfg(test)]
#[path = "aip_md_tests.rs"]
mod tests;

// endregion: --- Tests
