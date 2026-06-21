// region:    --- Tests

use super::*;
use crate::AipApiError;
use crate::impl_lua_serde_traits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct TestParams {
	data: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct TestResponse {
	data: String,
}

impl_lua_serde_traits!(TestParams);
impl_lua_serde_traits!(TestResponse);

impl crate::AipParams for TestParams {}
impl crate::AipOutput for TestResponse {}

fn test_sync_handler(params: TestParams) -> core::result::Result<TestResponse, AipApiError> {
	Ok(TestResponse { data: params.data })
}

async fn test_async_handler(params: TestParams) -> core::result::Result<TestResponse, AipApiError> {
	Ok(TestResponse { data: params.data })
}

// region:    --- Sync registration

#[test]
fn test_registry_register_sync_success() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::from_empty();

	// -- Exec
	registry.register_sync("test.parse", test_sync_handler)?;
	let fns = registry.list_registered_fns();

	// -- Check
	assert_eq!(fns.len(), 1);
	assert_eq!(fns[0].path, "test.parse");
	assert_eq!(fns[0].kind, AipFnKind::Sync);

	Ok(())
}

// endregion: --- Sync registration

// region:    --- Async registration

#[test]
fn test_registry_register_async_success() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::from_empty();

	// -- Exec
	registry.register_async("test.fetch", test_async_handler)?;
	let fns = registry.list_registered_fns();

	// -- Check
	assert_eq!(fns.len(), 1);
	assert_eq!(fns[0].path, "test.fetch");
	assert_eq!(fns[0].kind, AipFnKind::Async);

	Ok(())
}

// endregion: --- Async registration

// region:    --- Duplicate path

#[test]
fn test_registry_duplicate_path_rejection() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::from_empty();
	registry.register_sync("test.parse", test_sync_handler)?;

	// -- Exec
	let result = registry.register_sync("test.parse", test_sync_handler);

	// -- Check
	assert!(result.is_err(), "expected duplicate path error");
	let err = result.as_ref().unwrap_err();
	assert!(matches!(err, AipRegistryError::DuplicatePath(_)));

	Ok(())
}

// endregion: --- Duplicate path

// region:    --- Invalid paths

#[test]
fn test_registry_invalid_paths() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::from_empty();

	// -- Exec & Check
	assert!(registry.register_sync("", test_sync_handler).is_err());
	assert!(registry.register_sync("parse", test_sync_handler).is_err());
	assert!(registry.register_sync("test.", test_sync_handler).is_err());
	assert!(registry.register_sync(".parse", test_sync_handler).is_err());
	assert!(registry.register_sync("test..parse", test_sync_handler).is_err());

	Ok(())
}

// endregion: --- Invalid paths

// region:    --- Schema metadata

#[test]
fn test_registry_schema_metadata() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::from_empty();

	// -- Exec
	registry.register_sync("test.parse", test_sync_handler)?;
	let fns = registry.list_registered_fns();
	let registered = &fns[0];
	let params_schema = serde_json::to_value(&registered.params_schema)?;
	let response_schema = serde_json::to_value(&registered.response_schema)?;
	let error_schema = serde_json::to_value(&registered.error_schema)?;

	// -- Check
	assert!(!params_schema.is_null());
	assert!(!response_schema.is_null());
	assert!(!error_schema.is_null());

	Ok(())
}

// endregion: --- Schema metadata

// region:    --- Merge

#[test]
fn test_registry_merge_success() -> Result<()> {
	let mut reg1 = AipRegistry::from_empty();
	reg1.register_sync("test.parse", test_sync_handler)?;

	let mut reg2 = AipRegistry::from_empty();
	reg2.register_async("test.fetch", test_async_handler)?;

	// merge
	reg1.merge(reg2)?;

	let fns = reg1.list_registered_fns();
	assert_eq!(fns.len(), 2);
	assert!(fns.iter().any(|f| f.path == "test.parse"));
	assert!(fns.iter().any(|f| f.path == "test.fetch"));

	Ok(())
}

#[test]
fn test_registry_merge_duplicate() -> Result<()> {
	let mut reg1 = AipRegistry::from_empty();
	reg1.register_sync("test.parse", test_sync_handler)?;

	let mut reg2 = AipRegistry::from_empty();
	reg2.register_sync("test.parse", test_sync_handler)?;

	let result = reg1.merge(reg2);
	assert!(result.is_err());
	assert!(matches!(result.unwrap_err(), AipRegistryError::DuplicatePath(_)));

	Ok(())
}

// endregion: --- Merge

// endregion: --- Tests
