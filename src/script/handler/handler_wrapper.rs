use super::{AipParams, AipResponse};
use crate::script::{Handler, HandlerError, LuaJsonExt, PinFutureValue, handler_error_to_lua};
use mlua::{Lua, Value};
use std::marker::PhantomData;

/// `HandlerWrapper` is a `Handler` wrapper that implements
/// `HandlerWrapperTrait` for type erasure, enabling dynamic dispatch.
///
/// Modeled on `rpc-router::RpcHandlerWrapper`, but adapted to the single
/// params-table shape used by `AipFn`. The boundary now uses `mlua::Value`.
///
/// Generics:
/// - `H`: the concrete handler (function or closure) implementing `Handler`.
/// - `P`: the typed params (`FromLua`).
/// - `R`: the typed response (`ToLua`).
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
	H: Handler<P, R, M> + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
	/// Convert the Lua value to typed params, then call the wrapped handler.
	/// `FromLua` conversion happens synchronously here (outside the async block)
	/// so that `Lua` and `Value` are never captured across an await point,
	/// preserving the `Send` bound on the returned future.
	pub fn call(&self, lua: &Lua, params_value: Value) -> PinFutureValue {
		let params = match P::from_lua(lua, params_value) {
			Ok(p) => p,
            Err(err) => return Box::pin(async move { Err(crate::script::HandlerError::from(err)) }),
		};
		let handler = self.handler.clone();
		Box::pin(handler.call(lua.clone(), params))
	}
}

/// `HandlerWrapperTrait` enables `HandlerWrapper` to become a trait object,
/// allowing for dynamic dispatch by the registry.
pub trait HandlerWrapperTrait: Send + Sync {
	/// Call the wrapped handler with a Lua state and a Lua value params argument,
	/// returning a pinned future resolving to a Lua value response or a
	/// normalized `HandlerError`.
	fn call(&self, lua: &Lua, params_value: Value) -> PinFutureValue;
}

impl<H, P, R, M> HandlerWrapperTrait for HandlerWrapper<H, P, R, M>
where
	H: Handler<P, R, M> + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	M: Send + Sync + 'static,
{
	fn call(&self, lua: &Lua, params_value: Value) -> PinFutureValue {
		self.call(lua, params_value)
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
	use crate::impl_lua_serde_traits;
	use crate::script::{AipApiError, IntoHandlerWrapper, handler_error_to_lua};
use crate::script::LuaJsonExt;
	use schemars::JsonSchema;
	use serde::{Deserialize, Serialize};
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, Serialize, JsonSchema)]
	struct EchoParams {
		data: String,
	}

	#[derive(Debug, Deserialize, Serialize, JsonSchema)]
	struct EchoResult {
		data: String,
	}

	impl_lua_serde_traits!(EchoParams);
	impl_lua_serde_traits!(EchoResult);

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
		let lua = mlua::Lua::new();
	let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": "hello" }))
		.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let value = wrapper.call(&lua, params_lua).await.map_err(handler_error_to_lua)?;

		// -- Check
		let back_json =
            value.x_to_json_value().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		assert_eq!(back_json, Some(json!({ "data": "hello" })));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_wrapper_async_dyn_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let wrapper = echo_async.into_dyn();
		let lua = mlua::Lua::new();
	let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": "world" }))
		.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let value = wrapper.call(&lua, params_lua).await.map_err(handler_error_to_lua)?;

		// -- Check
		let back_json =
            value.x_to_json_value().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		assert_eq!(back_json, Some(json!({ "data": "world" })));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_wrapper_invalid_params_err() -> TestResult<()> {
		// -- Setup & Fixtures
		let wrapper = echo_sync.into_dyn();
		let lua = mlua::Lua::new();
		let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": 123 }))
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let res = wrapper.call(&lua, params_lua).await;

		// -- Check
		let herr = res.err().ok_or("should be an error")?;
		// After refactoring, the error is a HandlerError containing a string message
		// from the deserialization failure, rather than a typed AipApiError.
		assert!(herr.get::<String>().is_some(), "expected error message from invalid params");

		Ok(())
	}
}

// endregion: --- Tests
