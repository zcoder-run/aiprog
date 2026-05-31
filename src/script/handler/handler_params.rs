use crate::script::{AipApiError, HandlerError, IntoHandlerError};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Converts an incoming normalized params value (`serde_json::Value`) into a
/// typed `Params` value via `DeserializeOwned`.
///
/// This layer is Lua-agnostic. The Lua adapter is responsible for converting a
/// Lua params value into a normalized `serde_json::Value` (including converting
/// an empty Lua table into an empty JSON object) before it reaches this layer.
///
/// Behavior:
///
/// - A normalized JSON object is deserialized directly into the params type.
/// - An empty normalized value (`Value::Null` or an empty object) is treated as
///   an empty JSON object, allowing APIs with only optional fields to accept an
///   empty params table.
/// - Deserialization errors become a normalized `HandlerError` carrying an
///   `AipApiError` with code `INVALID_PARAMS`.
pub fn params_from_value<P>(value: Value) -> core::result::Result<P, HandlerError>
where
	P: DeserializeOwned,
{
	let value = normalize_params_value(value);

	serde_json::from_value(value).map_err(|err| {
		AipApiError::new("INVALID_PARAMS", "Failed to parse params")
			.with_cause(err.to_string())
			.into_handler_error()
	})
}

/// Normalizes an incoming params value so that an "empty" value becomes an empty
/// JSON object. This keeps the empty-object special case at the normalized-value
/// level, independent of Lua.
fn normalize_params_value(value: Value) -> Value {
	match value {
		Value::Null => Value::Object(serde_json::Map::new()),
		Value::Object(map) if map.is_empty() => Value::Object(serde_json::Map::new()),
		other => other,
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use serde::Deserialize;
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, PartialEq)]
	struct SampleParams {
		data: String,
	}

	#[derive(Debug, Default, Deserialize, PartialEq)]
	struct OptionalParams {
		#[serde(default)]
		mode: Option<String>,
	}

	#[test]
	fn test_handler_params_from_object_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let value = json!({ "data": "hello" });

		// -- Exec
		let params: SampleParams = params_from_value(value)?;

		// -- Check
		assert_eq!(
			params,
			SampleParams {
				data: "hello".to_string()
			}
		);

		Ok(())
	}

	#[test]
	fn test_handler_params_from_null_empty_ok() -> TestResult<()> {
		// -- Exec
		let params: OptionalParams = params_from_value(Value::Null)?;

		// -- Check
		assert_eq!(params, OptionalParams { mode: None });

		Ok(())
	}

	#[test]
	fn test_handler_params_from_empty_object_ok() -> TestResult<()> {
		// -- Exec
		let params: OptionalParams = params_from_value(json!({}))?;

		// -- Check
		assert_eq!(params, OptionalParams { mode: None });

		Ok(())
	}

	#[test]
	fn test_handler_params_invalid_err() {
		// -- Setup & Fixtures
		let value = json!({ "data": 123 });

		// -- Exec
		let res: core::result::Result<SampleParams, HandlerError> = params_from_value(value);

		// -- Check
		let err = res.expect_err("should be an error");
		let api_err = err.get::<AipApiError>().expect("should hold AipApiError");
		assert_eq!(api_err.code, "INVALID_PARAMS");
	}
}

// endregion: --- Tests
