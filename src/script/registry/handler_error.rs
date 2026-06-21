use crate::script::ScriptError;
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
	AipApi(AipApiError),

	/// Registry‑specific error (e.g., unknown method, duplicate path).
	// Registry(RegistryError),

	/// Script‑level error from Lua operations or conversion failures.
	Script(ScriptError),

	/// Fallback variant for string errors.
	Custom(String),
}

impl core::fmt::Display for HandlerError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			HandlerError::AipApi(e) => write!(f, "{e}"),
			HandlerError::Script(e) => write!(f, "{e}"),
			HandlerError::Custom(s) => f.write_str(s),
		}
	}
}

impl std::error::Error for HandlerError {}

/// Convert a normalized `HandlerError` into an `mlua::Error`.
///
/// When the handler error carries a typed `AipApiError`, the error code,
/// message, and optional details/cause are surfaced. A `RegistryError` is
/// surfaced with its display message. Otherwise, the error type name is used as
/// a fallback.
pub fn handler_error_to_lua(err: HandlerError) -> mlua::Error {
	match err {
		HandlerError::AipApi(api_err) => api_error_to_lua(api_err),
		HandlerError::Script(script_err) => mlua::Error::RuntimeError(script_err.to_string()),
		HandlerError::Custom(s) => mlua::Error::RuntimeError(s),
	}
}

fn api_error_to_lua(api_err: AipApiError) -> mlua::Error {
	let mut msg = format!("[{}] {}", api_err.code, api_err.message);
	if let Some(details) = api_err.details.as_ref() {
		msg.push_str(&format!("\nDetails: {details}"));
	}
	if let Some(cause) = api_err.cause.as_ref() {
		msg.push_str(&format!("\nCause: {cause}"));
	}
	mlua::Error::RuntimeError(msg)
}

// region:    --- IntoHandlerError

/// Trait for converting an application error into a `HandlerError`.
pub trait IntoHandlerError
where
	Self: Sized + Send + Sync + 'static,
{
	fn into_handler_error(self) -> HandlerError;
}

impl IntoHandlerError for HandlerError {
	fn into_handler_error(self) -> HandlerError {
		self
	}
}

impl IntoHandlerError for AipApiError {
	fn into_handler_error(self) -> HandlerError {
		HandlerError::AipApi(self)
	}
}

impl IntoHandlerError for ScriptError {
	fn into_handler_error(self) -> HandlerError {
		HandlerError::Script(self)
	}
}

impl IntoHandlerError for String {
	fn into_handler_error(self) -> HandlerError {
		HandlerError::Custom(self)
	}
}

impl IntoHandlerError for &'static str {
	fn into_handler_error(self) -> HandlerError {
		HandlerError::Custom(self.into())
	}
}

impl IntoHandlerError for serde_json::Value {
	fn into_handler_error(self) -> HandlerError {
		HandlerError::Custom(self.to_string())
	}
}

// endregion: --- IntoHandlerError

// region:    --- AipApiError

pub type AipApiResult<T> = core::result::Result<T, AipApiError>;

/// The standard typed API error.
///
/// Its primary contract is conversion into the normalized `HandlerError` via
/// `IntoHandlerError`. Conversion into `mlua::Error` happens only at the Lua
/// adapter layer.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipApiError {
	pub code: String,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cause: Option<String>,
}

impl AipApiError {
	/// Convenience constructor for the common case of a code and message.
	pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
		AipApiError {
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

impl core::fmt::Display for AipApiError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "[{}] {}", self.code, self.message)
	}
}

impl std::error::Error for AipApiError {}
impl crate::AipError for AipApiError {}

// endregion: --- AipApiError

// region:    --- From conversions

impl From<ScriptError> for HandlerError {
	fn from(err: ScriptError) -> Self {
		HandlerError::Script(err)
	}
}

impl From<crate::Error> for HandlerError {
	fn from(e: crate::Error) -> Self {
		match e {
			crate::Error::Handler(h) => h,
			crate::Error::AipApi(api_err) => HandlerError::AipApi(api_err),
			other => HandlerError::Custom(other.to_string()),
		}
	}
}

// endregion: --- From conversions

impl From<crate::Error> for AipApiError {
	fn from(e: crate::Error) -> Self {
		match e {
			crate::Error::AipApi(api_err) => api_err,
			other => AipApiError::new("INTERNAL_ERROR", other.to_string()),
		}
	}
}

// region:    --- Error Boilerplate

// (already implemented above, keep for consistency)
// endregion: --- Error Boilerplate
