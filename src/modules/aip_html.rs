//! Defines the `html` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.html` module provides functions for processing HTML content.
//!
//! ### Functions
//!
//! - `aip.html.slim(params: { html: string }) -> string`
//! - `aip.html.to_md(params: { html: string }) -> string`
//!
//! ---
//!

use crate::LuaExt;
use crate::registry::HandlerResult;
use crate::{AipFromLua, AipIntoLua, AipParams};
use crate::{AipOutput, AipRegistry};
use mlua::Lua;
use aiprog_macros::{aip_handler, register_handler};

/// Build and return an [`AipRegistry`] containing all `aip.html` handlers.
pub fn init_registry() -> crate::Result<AipRegistry> {
	let mut registry = AipRegistry::from_empty();
	register_handler!(registry, "aip.html.slim", aip_html_slim_handler)?;
	register_handler!(registry, "aip.html.to_md", aip_html_to_md_handler)?;
	Ok(registry)
}

// region:    --- aip.html.slim

/// Parameters for `aip.html.slim`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipHtmlSlimParams {
	/// The HTML string to slim.
	pub html: String,

	/// The indent number of chars (spaces)
	pub indent: Option<i64>,
}

impl AipFromLua for AipHtmlSlimParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let html = table
			.x_get_string("html")
			.ok_or_else(|| crate::Error::custom("Missing 'html' field"))?;

		let indent = table.x_get_i64("indent");

		Ok(AipHtmlSlimParams { html, indent })
	}
}

impl AipParams for AipHtmlSlimParams {}

/// Output type for `aip.html.slim`.
///
/// The slimmed HTML string is returned directly to Lua as a string.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipHtmlSlimOutput(pub String);

impl AipIntoLua for AipHtmlSlimOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		self.0.into_lua(lua)
	}
}

impl AipOutput for AipHtmlSlimOutput {}

/// Slims an HTML string by removing extra whitespace and indentation, returning the slimmed result.
#[aip_handler]
fn aip_html_slim_handler(params: AipHtmlSlimParams) -> HandlerResult<AipHtmlSlimOutput> {
	let indent = params.indent.unwrap_or(2);
	let opts = htmlr::SlimOptions::from_indent(indent as u8);
	let slimmed = htmlr::slim(&params.html, opts).map_err(|e| format!("aip.html.slim failed. {e}"))?;
	Ok(AipHtmlSlimOutput(slimmed))
}

// endregion: --- aip.html.slim

// region:    --- aip.html.to_md

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipHtmlToMdParams {
	/// The HTML string to convert to Markdown.
	pub html: String,
}

impl AipFromLua for AipHtmlToMdParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let html = table
			.x_get_string("html")
			.ok_or_else(|| crate::Error::custom("Missing 'html' field"))?;

		Ok(AipHtmlToMdParams { html })
	}
}

impl AipParams for AipHtmlToMdParams {}

/// Output type for `aip.html.to_md`.
///
/// The Markdown string is returned directly to Lua as a string.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipHtmlToMdOutput(pub String);

impl AipIntoLua for AipHtmlToMdOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
		self.0.into_lua(lua)
	}
}

impl AipOutput for AipHtmlToMdOutput {}

/// Converts an HTML string to Markdown.
#[aip_handler]
fn aip_html_to_md_handler(params: AipHtmlToMdParams) -> HandlerResult<AipHtmlToMdOutput> {
	let md = htmlr::to_md(&params.html, None).map_err(|e| e.to_string())?;
	Ok(AipHtmlToMdOutput(md))
}

// endregion: --- aip.html.to_md

// region:    --- Tests

// #[cfg(test)]
// #[path = "aip_html_tests.rs"]
// mod tests;

// endregion: --- Tests
