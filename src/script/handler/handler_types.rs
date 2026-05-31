use crate::script::{HandlerError, params_from_value, response_to_value};
use super::{AipParams, AipResponse};
use std::future::Future;
use std::pin::Pin;

/// The pinned future returned by a handler call, resolving to a normalized
/// response (`serde_json::Value`) or a normalized `HandlerError`.
pub type PinFutureValue = Pin<Box<dyn Future<Output = core::result::Result<serde_json::Value, HandlerError>> + Send>>;

/// The generic, Lua-agnostic handler trait, modeled on `rpc-router::Handler`.
///
/// Key points:
/// - A handler is a plain Rust function or closure taking a single typed `P`
///   (params) argument and returning a typed `Result<R, E>`.
/// - The trait operates on normalized `serde_json::Value` at its public
///   boundary. Typed conversion happens inside the handler implementation
///   (params satisfy `AipParams`, response satisfy `AipResponse`, error via
///   `IntoHandlerError`).
/// - Both sync and async handler kinds are supported through the
///   `impl_handler!` macro implementations.
/// - The handler layer has no dependency on `mlua`.
///
/// Type parameters:
/// - `P` is the typed params (satisfies `AipParams`).
/// - `R` is the typed response (satisfies `AipResponse`).
/// - `M` is a marker type used to distinguish the sync and async
///   implementations during type resolution.
pub trait Handler<P, R, M>: Clone
where
    P: AipParams,
    R: AipResponse,
{
	/// The future type returned by calling this handler.
	type Future: Future<Output = core::result::Result<serde_json::Value, HandlerError>> + Send + 'static;

	/// Call the handler with normalized params (`serde_json::Value`) and return
	/// a future resolving to the normalized response or a normalized error.
	fn call(self, params_value: serde_json::Value) -> Self::Future;
}

// region:    --- Markers

/// Marker type for synchronous handler implementations.
pub struct SyncMarker;

/// Marker type for asynchronous handler implementations.
pub struct AsyncMarker;

// endregion: --- Markers

// region:    --- Re-export helpers

// Re-exported so the macro can reference them through this module path.
pub(crate) use super::{
	params_from_value as handler_params_from_value, response_to_value as handler_response_to_value,
};

// endregion: --- Re-export helpers
