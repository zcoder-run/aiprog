/// Macro generating the `Handler` implementations for the supported handler
/// signatures: a single typed params argument, in both sync and async forms.
///
/// Modeled on `rpc-router::impl_handler_pair`, but adapted to the single
/// params-table shape used by `AipFn`. The boundary stays on
/// `serde_json::Value`, with typed conversion performed inside the
/// implementation.
#[macro_export]
macro_rules! impl_aip_handlers {
	() => {
		// -- Sync handler: fn(P) -> Result<R, E>
		impl<F, P, R, E> $crate::script::Handler<P, R, $crate::script::SyncMarker> for F
		where
			F: FnOnce(P) -> core::result::Result<R, E> + Clone + Send + 'static,
            P: serde::de::DeserializeOwned + schemars::JsonSchema + Send + Sync + 'static,
            R: serde::Serialize + schemars::JsonSchema + Send + Sync + 'static,
			E: $crate::script::IntoHandlerError,
		{
			type Future = $crate::script::PinFutureValue;

			fn call(self, params_value: serde_json::Value) -> Self::Future {
				Box::pin(async move {
					let params: P = $crate::script::params_from_value(params_value)?;

					match self(params) {
						Ok(response) => $crate::script::response_to_value(response),
						Err(err) => Err($crate::script::IntoHandlerError::into_handler_error(err)),
					}
				})
			}
		}

		// -- Async handler: fn(P) -> Future<Output = Result<R, E>>
		impl<F, Fut, P, R, E> $crate::script::Handler<P, R, $crate::script::AsyncMarker> for F
		where
			F: FnOnce(P) -> Fut + Clone + Send + 'static,
            P: serde::de::DeserializeOwned + schemars::JsonSchema + Send + Sync + 'static,
            R: serde::Serialize + schemars::JsonSchema + Send + Sync + 'static,
			E: $crate::script::IntoHandlerError,
			Fut: core::future::Future<Output = core::result::Result<R, E>> + Send,
		{
			type Future = $crate::script::PinFutureValue;

			fn call(self, params_value: serde_json::Value) -> Self::Future {
				Box::pin(async move {
					let params: P = $crate::script::params_from_value(params_value)?;

					match self(params).await {
						Ok(response) => $crate::script::response_to_value(response),
						Err(err) => Err($crate::script::IntoHandlerError::into_handler_error(err)),
					}
				})
			}
		}
	};
}

impl_aip_handlers!();

// region:    --- Tests

#[cfg(test)]
mod tests {
		use schemars::JsonSchema;
	use crate::script::{AipApiError, Handler};
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
	async fn test_handler_sync_call_ok() -> TestResult<()> {
		// -- Exec
		let value = echo_sync.call(json!({ "data": "hello" })).await?;

		// -- Check
		assert_eq!(value, json!({ "data": "hello" }));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_async_call_ok() -> TestResult<()> {
		// -- Exec
		let value = echo_async.call(json!({ "data": "world" })).await?;

		// -- Check
		assert_eq!(value, json!({ "data": "world" }));

		Ok(())
	}

	#[tokio::test]
	async fn test_handler_sync_invalid_params_err() -> TestResult<()> {
		// -- Exec
		let res = echo_sync.call(json!({ "data": 123 })).await;

		// -- Check
		let err = res.err().ok_or("should be an error")?;
		let api_err = err.get::<AipApiError>().ok_or("should hold AipApiError")?;
		assert_eq!(api_err.code, "INVALID_PARAMS");

		Ok(())
	}
}

// endregion: --- Tests
