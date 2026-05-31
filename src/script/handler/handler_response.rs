use crate::script::{AipApiError, HandlerError, IntoHandlerError};
use serde::Serialize;
use serde_json::Value;

/// Converts a typed response (`Serialize`) into a normalized `serde_json::Value`.
///
/// Because response structs already contain the `data` field, the normalized
/// value has the expected root-level API shape. The Lua adapter converts this
/// normalized value into a Lua table.
///
/// Serialization errors become a normalized `HandlerError` carrying an
/// `AipApiError` with code `RESPONSE_SERIALIZE_FAILED`.
pub fn response_to_value<R>(response: R) -> core::result::Result<Value, HandlerError>
where
	R: Serialize,
{
	serde_json::to_value(&response).map_err(|err| {
		AipApiError::new("RESPONSE_SERIALIZE_FAILED", "Failed to serialize API response")
			.with_cause(err.to_string())
			.into_handler_error()
	})
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use serde::Serialize;
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Serialize)]
	struct SampleResult {
		data: String,
	}

	#[test]
	fn test_handler_response_to_value_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let response = SampleResult {
			data: "hello".to_string(),
		};

		// -- Exec
		let value = response_to_value(response)?;

		// -- Check
		assert_eq!(value, json!({ "data": "hello" }));

		Ok(())
	}

	#[test]
	fn test_handler_response_value_passthrough_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let response = json!({ "data": [1, 2, 3] });

		// -- Exec
		let value = response_to_value(response.clone())?;

		// -- Check
		assert_eq!(value, response);

		Ok(())
	}
}

// endregion: --- Tests
