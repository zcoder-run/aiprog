use crate::Result;
use crate::script::script_error;
use crate::script::{AipFromLua, AipIntoLua, HandlerError, LuaJsonExt};
use mlua::{Lua, Value};

/// Macro generating the `Handler` implementations for the supported handler
/// signatures: a single typed params argument, in both sync and async forms.
///
/// The `HandlerWrapper::call` method performs `FromLua` conversion outside the
/// async block and passes the pre-converted `P` to this macro. This keeps
/// `mlua::Lua` and `mlua::Value` out of the async state, preserving `Send`.
#[macro_export]
macro_rules! impl_aip_handlers {
	() => {
		// -- Sync handler: fn(P) -> Result<R, E>
		impl<F, P, R, E> $crate::script::Handler<P, R, $crate::script::SyncMarker> for F
		where
			F: FnOnce(P) -> core::result::Result<R, E> + Clone + Send + 'static,
			P: $crate::script::AipParams,
			R: $crate::script::AipResponse,
			E: $crate::script::AipError,
		{
			type Future = $crate::script::PinFutureValue;

			fn call(self, lua: mlua::Lua, params: P) -> Self::Future {
				Box::pin(async move {
					match self(params) {
						Ok(response) => response.into_lua(&lua).map_err($crate::script::HandlerError::from),
						Err(err) => Err($crate::script::IntoHandlerError::into_handler_error(err)),
					}
				})
			}
		}

		// -- Async handler: fn(P) -> Future<Output = Result<R, E>>
		impl<F, Fut, P, R, E> $crate::script::Handler<P, R, $crate::script::AsyncMarker> for F
		where
			F: FnOnce(P) -> Fut + Clone + Send + 'static,
			P: $crate::script::AipParams,
			R: $crate::script::AipResponse,
			E: $crate::script::AipError,
			Fut: core::future::Future<Output = core::result::Result<R, E>> + Send,
		{
			type Future = $crate::script::PinFutureValue;

			fn call(self, lua: mlua::Lua, params: P) -> Self::Future {
				Box::pin(async move {
					match self(params).await {
						Ok(response) => response.into_lua(&lua).map_err($crate::script::HandlerError::from),
						Err(err) => Err($crate::script::IntoHandlerError::into_handler_error(err)),
					}
				})
			}
		}
	};
}

// region:    --- Tuple FromLua/ToLua implementations

impl<A: AipFromLua> AipFromLua for (A,) {
	fn from_lua(lua: &Lua, value: Value) -> script_error::ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| script_error::ScriptError::custom("Expected table for tuple"))?;
		let val_0 = table.get(1).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok((A::from_lua(lua, val_0)?,))
	}
}

impl<A: AipIntoLua> AipIntoLua for (A,) {
	fn into_lua(self, lua: &Lua) -> script_error::ScriptResult<Value> {
		let table = lua
			.create_table()
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(1, self.0.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok(Value::Table(table))
	}
}

impl<A: AipFromLua, B: AipFromLua> AipFromLua for (A, B) {
	fn from_lua(lua: &Lua, value: Value) -> script_error::ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| script_error::ScriptError::custom("Expected table for tuple"))?;
		let val_0 = table.get(1).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_1 = table.get(2).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok((A::from_lua(lua, val_0)?, B::from_lua(lua, val_1)?))
	}
}

impl<A: AipIntoLua, B: AipIntoLua> AipIntoLua for (A, B) {
	fn into_lua(self, lua: &Lua) -> script_error::ScriptResult<Value> {
		let table = lua
			.create_table()
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(1, self.0.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(2, self.1.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok(Value::Table(table))
	}
}

impl<A: AipFromLua, B: AipFromLua, C: AipFromLua> AipFromLua for (A, B, C) {
	fn from_lua(lua: &Lua, value: Value) -> script_error::ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| script_error::ScriptError::custom("Expected table for tuple"))?;
		let val_0 = table.get(1).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_1 = table.get(2).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_2 = table.get(3).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok((
			A::from_lua(lua, val_0)?,
			B::from_lua(lua, val_1)?,
			C::from_lua(lua, val_2)?,
		))
	}
}

impl<A: AipIntoLua, B: AipIntoLua, C: AipIntoLua> AipIntoLua for (A, B, C) {
	fn into_lua(self, lua: &Lua) -> script_error::ScriptResult<Value> {
		let table = lua
			.create_table()
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(1, self.0.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(2, self.1.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(3, self.2.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok(Value::Table(table))
	}
}

impl<A: AipFromLua, B: AipFromLua, C: AipFromLua, D: AipFromLua> AipFromLua for (A, B, C, D) {
	fn from_lua(lua: &Lua, value: Value) -> script_error::ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| script_error::ScriptError::custom("Expected table for tuple"))?;
		let val_0 = table.get(1).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_1 = table.get(2).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_2 = table.get(3).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		let val_3 = table.get(4).map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok((
			A::from_lua(lua, val_0)?,
			B::from_lua(lua, val_1)?,
			C::from_lua(lua, val_2)?,
			D::from_lua(lua, val_3)?,
		))
	}
}

impl<A: AipIntoLua, B: AipIntoLua, C: AipIntoLua, D: AipIntoLua> AipIntoLua for (A, B, C, D) {
	fn into_lua(self, lua: &Lua) -> script_error::ScriptResult<Value> {
		let table = lua
			.create_table()
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(1, self.0.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(2, self.1.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(3, self.2.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		table
			.set(4, self.3.into_lua(lua)?)
			.map_err(|e| script_error::ScriptError::custom(e.to_string()))?;
		Ok(Value::Table(table))
	}
}

// endregion: --- Tuple FromLua/ToLua implementations

impl_aip_handlers!();

// region:    --- Tests

#[cfg(test)]
mod tests {
	use crate::script::LuaJsonExt;
	use crate::script::{AipApiError, Handler, HandlerError};
	use aiprog_macros::{AipFromLua, AipIntoLua, AipParams, AipResponse};
	use schemars::JsonSchema;
	use serde::{Deserialize, Serialize};
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, Serialize, JsonSchema, AipFromLua, AipIntoLua, AipParams)]
	struct EchoParams {
		data: String,
	}

	#[derive(Debug, Deserialize, Serialize, JsonSchema, AipFromLua, AipIntoLua, AipResponse)]
	struct EchoResult {
		data: String,
	}

	fn echo_sync(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	async fn echo_async(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	fn fail_sync(_params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Err(AipApiError::new("INTERNAL_ERROR", "boom"))
	}
	#[tokio::test]
	async fn test_handler_sync_call_ok() -> TestResult<()> {
		// -- Setup
		let lua = mlua::Lua::new();
		let params_json = json!({ "data": "hello" });
		let params = serde_json::from_value::<EchoParams>(params_json.clone())?;

		// -- Exec
		let lua_val = echo_sync.call(lua.clone(), params).await?;

		// -- Check
		let back_json = lua_val
			.x_to_json_value()
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		assert_eq!(back_json, Some(params_json));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_async_call_ok() -> TestResult<()> {
		// -- Setup
		let lua = mlua::Lua::new();
		let params_json = json!({ "data": "world" });
		let params = serde_json::from_value::<EchoParams>(params_json.clone())?;

		// -- Exec
		let lua_val = echo_async.call(lua.clone(), params).await?;

		// -- Check
		let back_json = lua_val
			.x_to_json_value()
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		assert_eq!(back_json, Some(params_json));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_sync_invalid_params_err() -> TestResult<()> {
		// -- Setup
		let lua = mlua::Lua::new();
		let params = EchoParams {
			data: "irrelevant".to_string(),
		};

		// -- Exec
		let result = fail_sync.call(lua.clone(), params).await;
		assert!(result.is_err());
		let err = result.unwrap_err();
		let HandlerError::AipApi(api_err) = err else {
			return Err(format!("expected AipApiError, got {err:?}").into());
		};
		assert_eq!(api_err.code, "INTERNAL_ERROR");

		Ok(())
	}
}

// endregion: --- Tests
