use std::borrow::Cow;

use lazy_regex::regex;
use serde::Serialize;

pub type HandlerResult<T> = core::result::Result<T, HandlerError>;

/// Normalized, Lua‑agnostic handler error.
///
/// An enum carrying concrete error types used throughout the handler layer.
/// This replaces the previous type‑erased container, making the error shape explicit.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum HandlerError {
	/// Structured API error (code, message, optional details/cause).
	Api(ApiError),

	/// Registry‑specific error (e.g., unknown method, duplicate path).
	// Registry(RegistryError),


	/// Fallback variant for string errors.
	Custom(String),
}

impl core::fmt::Display for HandlerError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			HandlerError::Api(e) => write!(f, "{e}"),
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
			HandlerError::Api(api_err) => api_error_to_lua(api_err),
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
}

fn api_error_to_lua(api_err: ApiError) -> mlua::Error {
	let mut msg = format!("[{}] {}", api_err.code, api_err.message);
	if let Some(details) = api_err.details.as_ref() {
		msg.push_str(&format!("\nDetails: {details}"));
	}
	if let Some(cause) = api_err.cause.as_ref() {
		msg.push_str(&format!("\nCause: {cause}"));
	}
	mlua::Error::RuntimeError(msg)
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

// region:    --- AipApiError

pub type ApiResult<T> = core::result::Result<T, ApiError>;

/// The standard typed API error.
///
/// Its primary contract is conversion into the normalized `HandlerError` via
/// `IntoHandlerError`. Conversion into `mlua::Error` happens only at the Lua
/// adapter layer.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct ApiError {
	pub code: String,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cause: Option<String>,
}

impl ApiError {
	/// Convenience constructor for the common case of a code and message.
	pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
		ApiError {
			code: code.into(),
			message: message.into(),
			details: None,
			cause: None,
		}
	}

	pub fn with_details(mut self, details: impl Into<String>) -> Self {
		self.details = Some(details.into());
		self
	}

	pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
		self.cause = Some(cause.into());
		self
	}
}

impl core::fmt::Display for ApiError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "[{}] {}", self.code, self.message)
	}
}

impl std::error::Error for ApiError {}
impl crate::AipError for ApiError {}

// endregion: --- AipApiError

// region:    --- From conversions

impl From<crate::Error> for HandlerError {
	fn from(e: crate::Error) -> Self {
		match e {
			crate::Error::Handler(h) => h,
			crate::Error::Api(api_err) => HandlerError::Api(api_err),
			other => HandlerError::Custom(other.to_string()),
		}
	}
}

impl From<ApiError> for HandlerError {
	fn from(e: ApiError) -> Self {
		HandlerError::Api(e)
	}
}

impl From<String> for HandlerError {
	fn from(s: String) -> Self {
		HandlerError::Custom(s)
	}
}

impl From<&str> for HandlerError {
	fn from(s: &str) -> Self {
		HandlerError::Custom(s.to_string())
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

impl From<crate::Error> for ApiError {
	fn from(e: crate::Error) -> Self {
		match e {
			crate::Error::Api(api_err) => api_err,
			other => ApiError::new("INTERNAL_ERROR", other.to_string()),
		}
	}
}

// region:    --- Error Boilerplate

// (already implemented above, keep for consistency)
// endregion: --- Error Boilerplate
