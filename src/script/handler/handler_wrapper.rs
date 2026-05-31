use super::{AipParams, AipResponse};
use crate::script::{Handler, HandlerError, PinFutureValue};
use std::marker::PhantomData;

/// `HandlerWrapper` is a `Handler` wrapper that implements
/// `HandlerWrapperTrait` for type erasure, enabling dynamic dispatch.
///
/// Modeled on `rpc-router::RpcHandlerWrapper`, but adapted to the single
/// params-table shape used by `AipFn`. The boundary stays fully Lua-agnostic
/// and operates only on normalized `serde_json::Value`.
///
/// Generics:
/// - `H`: the concrete handler (function or closure) implementing `Handler`.
/// - `P`: the typed params (`DeserializeOwned`).
/// - `R`: the typed response (`Serialize`).
/// - `M`: the marker type distinguishing the sync and async implementations.
///
/// All types except `H` match the generics of the `H` handler trait and are
/// kept in phantom data.
#[derive(Clone)]
pub struct HandlerWrapper<H, P, R, M> {
	handler: H,
	_marker: PhantomData<(P, R, M)>,
}

// Constructor
impl<H, P, R, M> HandlerWrapper<H, P, R, M> {
	pub fn new(handler: H) -> Self {
		Self {
			handler,
			_marker: PhantomData,
		}
	}
}

// Call Impl
impl<H, P, R, M> HandlerWrapper<H, P, R, M>
where
	H: Handler<P, R, M> + Clone + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
	pub fn call(&self, params_value: serde_json::Value) -> H::Future {
		// Note: Since the handler is `FnOnce`-like, we clone it so it can be
		//       called repeatedly. This is typically optimized by the compiler.
		let handler = self.handler.clone();
		Handler::call(handler, params_value)
	}
}

/// `HandlerWrapperTrait` enables `HandlerWrapper` to become a trait object,
/// allowing for dynamic dispatch by the registry.
pub trait HandlerWrapperTrait: Send + Sync {
	/// Call the wrapped handler with normalized params (`serde_json::Value`),
	/// returning a pinned future resolving to the normalized response or a
	/// normalized `HandlerError`.
	fn call(&self, params_value: serde_json::Value) -> PinFutureValue;
}

impl<H, P, R, M> HandlerWrapperTrait for HandlerWrapper<H, P, R, M>
where
	H: Handler<P, R, M> + Clone + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
	fn call(&self, params_value: serde_json::Value) -> PinFutureValue {
		Box::pin(self.call(params_value))
	}
}

/// Convenience extension to convert a concrete `Handler` into a boxed
/// `HandlerWrapperTrait` for dynamic dispatch by the registry.
pub trait IntoHandlerWrapper<P, R, M>: Handler<P, R, M>
where
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
	fn into_dyn(self) -> Box<dyn HandlerWrapperTrait>
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		Box::new(HandlerWrapper::new(self)) as Box<dyn HandlerWrapperTrait>
	}
}

impl<H, P, R, M> IntoHandlerWrapper<P, R, M> for H
where
	H: Handler<P, R, M> + Clone + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use crate::script::{AipApiError, IntoHandlerWrapper};
	use schemars::JsonSchema;
	use serde::{Deserialize, Serialize};
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, JsonSchema)]
	struct EchoParams {
		data: String,
	}

	#[derive(Debug, Serialize, JsonSchema)]
	struct EchoResult {
		data: String,
	}

	fn echo_sync(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	async fn echo_async(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	#[tokio::test]
	async fn test_handler_wrapper_sync_dyn_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let wrapper = echo_sync.into_dyn();

		// -- Exec
		let value = wrapper.call(json!({ "data": "hello" })).await?;

		// -- Check
		assert_eq!(value, json!({ "data": "hello" }));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_wrapper_async_dyn_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let wrapper = echo_async.into_dyn();

		// -- Exec
		let value = wrapper.call(json!({ "data": "world" })).await?;

		// -- Check
		assert_eq!(value, json!({ "data": "world" }));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_wrapper_invalid_params_err() -> TestResult<()> {
		// -- Setup & Fixtures
		let wrapper = echo_sync.into_dyn();

		// -- Exec
		let res = wrapper.call(json!({ "data": 123 })).await;

		// -- Check
		let err = res.err().ok_or("should be an error")?;
		let api_err = err.get::<AipApiError>().ok_or("should hold AipApiError")?;
		assert_eq!(api_err.code, "INVALID_PARAMS");

		Ok(())
	}
}

// endregion: --- Tests
