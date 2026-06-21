use super::IntoHandlerError;
use crate::{AipFromLua, AipIntoLua};
use schemars::JsonSchema;

// region:    --- AipParams

/// Unified trait for handler params types.
///
/// Any type used as handler params must be deserializable from a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipParams: AipFromLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipParams

// region:    --- AipOutput

/// Unified trait for handler output types.
///
/// Any type used as a handler output must be serializable to a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipOutput: AipIntoLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipOutput

// region:    --- AipError

/// Unified trait for handler error types.
///
/// Any type used as a handler error must be convertible into a normalized
/// `HandlerError`, have a JSON schema, and be `Send`. The blanket
/// implementation ensures any type satisfying the component bounds
/// automatically qualifies.
pub trait AipError: IntoHandlerError + JsonSchema + Send + 'static {}

// endregion: --- AipError
