use derive_more::{Display, From};
use lazy_regex::regex;
use serde::Serialize;
use std::borrow::Cow;

pub type HandlerResult<T> = core::result::Result<T, HandlerError>;

/// Normalized, Lua‑agnostic handler error.
///
/// Simple string-based error carrying a user-facing message.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema, From)]
pub enum HandlerError {
	/// Fallback variant for string errors.
	#[from(String, &String, &str)]
	Custom(String),
}

impl core::fmt::Display for HandlerError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			HandlerError::Custom(s) => f.write_str(s),
		}
	}
}

impl std::error::Error for HandlerError {}

impl HandlerError {
	/// Convert a normalized `HandlerError` into an `mlua::Error`.
	///
	/// When the handler error carries a typed `AipApiError`, the error code,
	/// message, and optional details/cause are surfaced. A `RegistryError` is
	/// surfaced with its display message. Otherwise, the error type name is used as
	/// a fallback.
	pub fn into_lua_error(self) -> mlua::Error {
		match self {
			HandlerError::Custom(s) => mlua::Error::RuntimeError(s),
		}
	}

	/// Build a `HandlerError` from a Lua error, enriching stack traces with the provided script source.
	pub fn from_lua_error_with_script(lua_error: &mlua::Error, script: &str) -> Self {
		let mut buff: Vec<String> = Vec::new();
		for item in lua_error.chain() {
			buff.push(process_stack_with_script(&item.to_string(), script));
		}
		HandlerError::Custom(buff.join("\n"))
	}

	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}

	pub fn cc(context: impl Into<String>, cause: impl std::fmt::Display) -> Self {
		Self::Custom(format!("{}: {}", context.into(), cause))
	}
}

// region:    --- Private helpers

fn process_stack_with_script(stack: &str, script: &str) -> String {
	let script_lines: Vec<&str> = script.lines().collect();
	let mut buff: Vec<Cow<str>> = Vec::new();
	let rx = regex!(r#"src/script/lua_engine\s*\.[^\n]*:(\d+):"#);
	for line in stack.lines() {
		if rx.is_match(line) {
			let replaced_line = rx.replace_all(line, |caps: &regex::Captures| {
				if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
					if let Some(script_line) = script_lines.get(num - 1) {
						let script_line = script_line.trim();
						Cow::from(format!("At line {num} '{script_line}'"))
					} else {
						Cow::from(format!("Line({num})"))
					}
				} else {
					Cow::from("")
				}
			});
			buff.push(replaced_line);
		} else {
			buff.push(line.into());
		}
	}
	buff.join("\n")
}

// endregion: --- Private helpers

// region:    --- From conversions

impl From<crate::Error> for HandlerError {
	fn from(e: crate::Error) -> Self {
		HandlerError::Custom(e.to_string())
	}
}

impl From<serde_json::Value> for HandlerError {
	fn from(v: serde_json::Value) -> Self {
		HandlerError::Custom(v.to_string())
	}
}

impl From<mlua::Error> for HandlerError {
	fn from(e: mlua::Error) -> Self {
		HandlerError::Custom(e.to_string())
	}
}

// endregion: --- From conversions

// region:    --- Error Boilerplate

// (already implemented above, keep for consistency)
// endregion: --- Error Boilerplate
