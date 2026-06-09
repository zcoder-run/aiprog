use super::IntoHandlerError;
use crate::script::{AipFromLua, AipIntoLua};
use schemars::JsonSchema;

// region:    --- AipParams

/// Unified trait for handler params types.
///
/// Any type used as handler params must be deserializable from a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipParams: AipFromLua + JsonSchema + Send + Sync + 'static {}

impl<T> AipParams for T where T: AipFromLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipParams

// region:    --- AipResponse

/// Unified trait for handler response types.
///
/// Any type used as a handler response must be serializable to a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipResponse: AipIntoLua + JsonSchema + Send + Sync + 'static {}

impl<T> AipResponse for T where T: AipIntoLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipResponse

// region:    --- AipError

/// Unified trait for handler error types.
///
/// Any type used as a handler error must be convertible into a normalized
/// `HandlerError`, have a JSON schema, and be `Send`. The blanket
/// implementation ensures any type satisfying the component bounds
/// automatically qualifies.
pub trait AipError: IntoHandlerError + JsonSchema + Send + 'static {}

impl<T> AipError for T where T: IntoHandlerError + JsonSchema + Send + 'static {}

// endregion: --- AipError
