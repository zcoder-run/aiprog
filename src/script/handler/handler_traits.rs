use super::IntoHandlerError;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;

// region:    --- AipParams

/// Unified trait for handler params types.
///
/// Any type used as handler params must be deserializable from JSON, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipParams: DeserializeOwned + JsonSchema + Send + Sync + 'static {}

impl<T> AipParams for T where T: DeserializeOwned + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipParams

// region:    --- AipResponse

/// Unified trait for handler response types.
///
/// Any type used as a handler response must be serializable to JSON, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipResponse: Serialize + JsonSchema + Send + Sync + 'static {}

impl<T> AipResponse for T where T: Serialize + JsonSchema + Send + Sync + 'static {}

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
