//! Script-aware error details for a failed Lua execution.
//!
//! `LuaErrorDetails` is built by the engine layer (`ScriptEngine::exec` / `exec_raw`),
//! which is the only layer that knows the script source. It is carried by
//! `crate::Error::LuaScript`.

use lazy_regex::regex;
use serde::Serialize;
use std::fmt;
use std::sync::Arc;

/// Number of lines shown before and after the failing line in `surround_code`.
const SURROUND_LINES: usize = 2;

/// Script-aware error container carried by `Error::LuaScript`.
///
/// The full script is retained via a cheap `Arc<str>` for consumers that need the
/// whole source, while `surround_code` is precomputed at construction time so that
/// `Display` and serialization do not need to re-derive it.
#[derive(Debug, Clone, Serialize)]
pub struct LuaErrorDetails {
	#[serde(serialize_with = "serialize_script")]
	script: Arc<str>,
	line_number: Option<u32>,
	/// Couple of lines before and after the failing line, with a `>` marker on it.
	surround_code: Option<String>,
	/// The handler/registry-composed message (handler path prefix plus the human-readable cause).
	message: String,
	/// The Lua stack traceback, if available.
	#[serde(skip_serializing_if = "Option::is_none")]
	stack_trace: Option<String>,
}

// region:    --- Constructors

impl LuaErrorDetails {
	/// Create the details from the full script, an optional failing line number, the message,
	/// and an optional stack traceback.
	///
	/// `surround_code` is derived from `script` and `line_number` at construction time.
	pub fn new(
		script: impl Into<Arc<str>>,
		line_number: Option<u32>,
		message: impl Into<String>,
		stack_trace: Option<String>,
	) -> Self {
		let script = script.into();
		let surround_code = line_number.and_then(|num| build_surround_code(&script, num));
		Self {
			script,
			line_number,
			surround_code,
			message: message.into(),
			stack_trace,
		}
	}

	/// Build the details from an `mlua::Error` and the executed script source.
	///
	/// The engine loads scripts with the chunk name `=script`, so error locations
	/// look like `script:12:`. The first such location found while walking the
	/// error chain is used as the failing line number.
	///
	/// The error message and stack traceback are split at the `stack traceback:`
	/// boundary: `message` contains only the human-readable cause, while
	/// `stack_trace` carries the traceback block (if present).
	pub fn from_lua_error(lua_error: &mlua::Error, script: impl Into<Arc<str>>) -> Self {
		let rx = regex!(r"\bscript:(\d+):");
		let mut line_number: Option<u32> = None;
		let mut message: Option<String> = None;
		let mut stack_trace: Option<String> = None;
		for item in lua_error.chain() {
			let item_str = item.to_string();
			if line_number.is_none() {
				line_number = rx
					.captures(&item_str)
					.and_then(|caps| caps.get(1))
					.and_then(|m| m.as_str().parse::<u32>().ok());
			}
			// Split at the `stack traceback:` boundary: text before forms the
			// error message, text from `stack traceback:` onward forms the traceback.
			if let Some(tb_idx) = item_str.find("stack traceback:") {
				if message.is_none() {
					message = Some(item_str[..tb_idx].trim_end().to_string());
				}
				if stack_trace.is_none() {
					stack_trace = Some(item_str[tb_idx..].to_string());
				}
			} else if message.is_none() {
				message = Some(item_str);
			}
		}
		Self::new(script, line_number, message.unwrap_or_default(), stack_trace)
	}
}

// endregion: --- Constructors

// region:    --- Accessors

impl LuaErrorDetails {
	pub fn script(&self) -> &str {
		&self.script
	}

	pub fn line_number(&self) -> Option<u32> {
		self.line_number
	}

	pub fn surround_code(&self) -> Option<&str> {
		self.surround_code.as_deref()
	}

	pub fn message(&self) -> &str {
		&self.message
	}

	pub fn stack_trace(&self) -> Option<&str> {
		self.stack_trace.as_deref()
	}
}

// endregion: --- Accessors

// region:    --- Display

impl fmt::Display for LuaErrorDetails {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.message)?;
		if let Some(line_number) = self.line_number {
			write!(f, "\n  at line {line_number}:")?;
			if let Some(surround_code) = &self.surround_code {
				write!(f, "\n{surround_code}")?;
			}
		}
		if let Some(stack_trace) = &self.stack_trace {
			write!(f, "\n{stack_trace}")?;
		}
		Ok(())
	}
}

// endregion: --- Display

// region:    --- Support

/// Build the annotated source block around the failing line, e.g.:
///
/// ```text
///     11 | local data = load_input()
///  >  12 | local r = aip.json.stringify({ data = data, pretty = "yes" })
///     13 | return r
/// ```
fn build_surround_code(script: &str, line_number: u32) -> Option<String> {
	let lines: Vec<&str> = script.lines().collect();
	let line_idx = (line_number as usize).checked_sub(1)?;
	if line_idx >= lines.len() {
		return None;
	}

	let start = line_idx.saturating_sub(SURROUND_LINES);
	let end = (line_idx + SURROUND_LINES + 1).min(lines.len());
	let num_width = end.to_string().len();

	let mut buff: Vec<String> = Vec::with_capacity(end - start);
	for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
		let num = idx + 1;
		let marker = if idx == line_idx { ">" } else { " " };
		buff.push(format!("  {marker} {num:>num_width$} | {line}"));
	}

	Some(buff.join("\n"))
}

/// Serialize the `Arc<str>` script as a plain string (avoids requiring the serde `rc` feature).
fn serialize_script<S: serde::Serializer>(script: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(script)
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_error_lua_details_new_surround_code() -> Result<()> {
		// -- Setup & Fixtures
		let script = "line one\nline two\nline three\nline four\nline five";

		// -- Exec
		let details = LuaErrorDetails::new(script, Some(3), "some failure message", None);

		// -- Check
		assert_eq!(details.line_number(), Some(3));
		assert_eq!(details.message(), "some failure message");
		let surround = details.surround_code().ok_or("Should have surround_code")?;
		assert!(surround.contains("    1 | line one"));
		assert!(surround.contains("  > 3 | line three"));
		assert!(surround.contains("    5 | line five"));

		Ok(())
	}

	#[test]
	fn test_error_lua_details_new_no_line() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local a = 1\nreturn a";

		// -- Exec
		let details = LuaErrorDetails::new(script, None, "plain message", None);

		// -- Check
		assert_eq!(details.line_number(), None);
		assert!(details.surround_code().is_none());

		Ok(())
	}

	#[test]
	fn test_error_lua_details_display_with_line() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local a = 1\nlocal b = bad()\nreturn a";
		let details = LuaErrorDetails::new(script, Some(2), "aip.test - some failure", None);

		// -- Exec
		let rendered = details.to_string();

		// -- Check
		assert!(rendered.starts_with("aip.test - some failure"));
		assert!(rendered.contains("at line 2:"));
		assert!(rendered.contains("  > 2 | local b = bad()"));
		assert!(rendered.contains("    1 | local a = 1"));
		assert!(rendered.contains("    3 | return a"));

		Ok(())
	}

	#[test]
	fn test_error_lua_details_display_no_line() -> Result<()> {
		// -- Setup & Fixtures
		let details = LuaErrorDetails::new("local a = 1", None, "plain message", None);

		// -- Exec
		let rendered = details.to_string();

		// -- Check
		assert_eq!(rendered, "plain message");

		Ok(())
	}

	#[test]
	fn test_error_lua_details_from_lua_error_simple() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local a = 1\nerror('boom')\nreturn a";
		let lua_err = mlua::Error::RuntimeError("script:2: boom".to_string());

		// -- Exec
		let details = LuaErrorDetails::from_lua_error(&lua_err, script);

		// -- Check
		assert_eq!(details.line_number(), Some(2));
		let surround = details.surround_code().ok_or("Should have surround_code")?;
		assert!(surround.contains("  > 2 | error('boom')"));
		assert!(details.message().contains("boom"));
		assert!(details.stack_trace().is_none());

		Ok(())
	}

	#[test]
	fn test_error_lua_details_from_lua_error_with_traceback() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local a = 1\nlocal b = bad()\nreturn a";
		let lua_err = mlua::Error::RuntimeError(
			"script:2: boom\nstack traceback:\n\t[C]: in local 'poll'\n\tscript:2: in main chunk".to_string(),
		);

		// -- Exec
		let details = LuaErrorDetails::from_lua_error(&lua_err, script);

		// -- Check
		assert_eq!(details.line_number(), Some(2));
		assert!(details.message().contains("boom"));
		assert!(!details.message().contains("stack traceback:"));
		let stack_trace = details.stack_trace().ok_or("Should have stack_trace")?;
		assert!(stack_trace.contains("stack traceback:"));
		assert!(stack_trace.contains("[C]: in local 'poll'"));

		Ok(())
	}

	#[test]
	fn test_error_lua_details_display_with_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local a = 1\nlocal b = bad()\nreturn a";
		let lua_err = mlua::Error::RuntimeError(
			"script:2: boom\nstack traceback:\n\t[C]: in local 'poll'\n\tscript:2: in main chunk".to_string(),
		);
		let details = LuaErrorDetails::from_lua_error(&lua_err, script);

		// -- Exec
		let rendered = details.to_string();

		// -- Check
		assert!(rendered.starts_with("runtime error: script:2: boom"));
		assert!(rendered.contains("at line 2:"));
		assert!(rendered.contains("  > 2 | local b = bad()"));
		let surround_pos = rendered.find("  > 2 |").ok_or("Should have surround code")?;
		let tb_pos = rendered.find("stack traceback:").ok_or("Should have stack traceback in display")?;
		assert!(tb_pos > surround_pos, "stack traceback should come after surround code");

		Ok(())
	}
}

// endregion: --- Tests
