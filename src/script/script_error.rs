use derive_more::{Display, From};
use lazy_regex::regex;

use crate::script::HandlerError;
use std::borrow::Cow;

pub type ScriptResult<T> = core::result::Result<T, ScriptError>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum ScriptError {
	#[from(String, &String, &str)]
	Custom(String),
}

// region:    --- Custom

impl ScriptError {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}

	/// Convert a Lua error chain into a `ScriptError`, enriching the stack trace with script context.
	pub fn from_error_with_script(lua_error: &mlua::Error, script: &str) -> Self {
		let mut buff: Vec<String> = Vec::new();
		for item in lua_error.chain() {
			if let Some(lua_item) = item.downcast_ref::<mlua::Error>() {
				let msg = lua_item.to_string();
				let msg = if msg.contains("traceback") | msg.contains("syntax") {
					process_stack_with_script(&msg, script)
				} else {
					msg
				};
				buff.push(format!("Lua error:\n{msg}"));
			} else {
				buff.push(format!("Other lua error:\n{item}"));
			}
		}
		ScriptError::custom(buff.join("\n"))
	}
}

// endregion: --- Custom

// region:    --- Private helpers

fn process_stack_with_script(stack: &str, script: &str) -> String {
	let script_lines: Vec<&str> = script.lines().collect();
	let mut buff: Vec<Cow<str>> = Vec::new();

	let rx = regex!(r#"src/script/lua_engine\s*\.[^\n]*:(\d+):"#);

	for line in stack.lines() {
		if rx.is_match(line) {
			// Replace all occurrences of the pattern with the extracted number
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
			// Add the original line if no match is found
			buff.push(line.into());
		}
	}

	buff.join("\n")
}

// endregion: --- Private helpers

// region:    --- Conversions

impl From<HandlerError> for ScriptError {
	fn from(handler_error: HandlerError) -> Self {
		ScriptError::custom(handler_error.to_string())
	}
}

// endregion: --- Conversions

// region:    --- Error Boilerplate

impl std::error::Error for ScriptError {}

// endregion: --- Error Boilerplate

// region:    --- Conversions

impl From<mlua::Error> for ScriptError {
	fn from(err: mlua::Error) -> Self {
		ScriptError::Custom(err.to_string())
	}
}

// endregion: --- Conversions
