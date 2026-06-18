use super::*;
use crate::impl_lua_serde_traits;
use crate::registry::AipRegistry;
use crate::script::AipApiError;
use mlua::Value;
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

impl crate::script::AipParams for TestParams {}
impl crate::script::AipResponse for TestResponse {}

fn test_sync_handler(params: TestParams) -> core::result::Result<TestResponse, AipApiError> {
	Ok(TestResponse { data: params.data })
}

async fn test_async_handler(params: TestParams) -> core::result::Result<TestResponse, AipApiError> {
	Ok(TestResponse { data: params.data })
}

fn test_error_handler(_params: TestParams) -> core::result::Result<TestResponse, AipApiError> {
	Err(AipApiError {
		code: "TEST_ERROR".into(),
		message: "forced test error".into(),
		details: Some("detail test".into()),
		cause: None,
	})
}

// region:    --- Nested table creation

#[test]
fn test_script_engine_nested_table_creation() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::default();
	registry.register_sync("aip.test.my_func", test_sync_handler)?;

	// -- Exec
	let engine = ScriptEngine::from_registry(registry)?;

	// -- Check
	let lua = engine.lua();
	let value: Value = lua.load("return aip.test.my_func({data='hello'})").eval()?;
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "hello");

	Ok(())
}

// endregion: --- Nested table creation

// region:    --- Existing compatible intermediate table

#[test]
fn test_script_engine_existing_compatible_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	let aip = lua.create_table()?;
	let existing = lua.create_table()?;
	aip.set("existing", existing.clone())?;
	lua.globals().set("aip", aip)?;

	let mut registry = AipRegistry::default();
	registry.register_sync("aip.existing.my_func", test_sync_handler)?;

	// -- Exec
	engine.register(registry)?;

	// -- Check
	let value: Value = lua.load("return aip.existing.my_func({data='world'})").eval()?;
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "world");

	Ok(())
}

// endregion: --- Existing compatible intermediate table

// region:    --- Intermediate non-table conflict

#[test]
fn test_script_engine_intermediate_non_table_conflict() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// Set aip.existing to a number instead of table
	lua.globals().set("aip", lua.create_table()?)?;
	let aip_table: mlua::Table = lua.globals().get("aip")?;
	aip_table.set("existing", 42)?;

	let mut registry = AipRegistry::default();
	registry.register_sync("aip.existing.my_func", test_sync_handler)?;

	// -- Exec
	let result = engine.register(registry);

	// -- Check
	assert!(result.is_err());
	let err_str = result.as_ref().unwrap_err().to_string();
	assert!(err_str.contains("not a table"));

	Ok(())
}

// endregion: --- Intermediate non-table conflict

// region:    --- Leaf conflict

#[test]
fn test_script_engine_leaf_conflict() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// Create aip.conflict table and a function at my_func
	let aip = lua.create_table()?;
	let conflict = lua.create_table()?;
	conflict.set(
		"my_func",
		lua.create_function(|_, _: mlua::Value| Ok(mlua::Value::Nil))?,
	)?;
	aip.set("conflict", conflict)?;
	lua.globals().set("aip", aip)?;

	let mut registry = AipRegistry::default();
	registry.register_sync("aip.conflict.my_func", test_sync_handler)?;

	// -- Exec
	let result = engine.register(registry);

	// -- Check
	assert!(result.is_err());
	let err_str = result.as_ref().unwrap_err().to_string();
	assert!(err_str.contains("already exists"));

	Ok(())
}

// endregion: --- Leaf conflict

// region:    --- Sync invocation

#[test]
fn test_script_engine_sync_invocation() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::default();
	registry.register_sync("aip.test.echo", test_sync_handler)?;
	let engine = ScriptEngine::from_registry(registry)?;

	// -- Exec
	let value: Value = engine.lua().load("return aip.test.echo({data='sync test'})").eval()?;

	// -- Check
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "sync test");

	Ok(())
}

// endregion: --- Sync invocation

// region:    --- Sync handler error conversion

#[test]
fn test_script_engine_sync_error_conversion() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::default();
	registry.register_sync("aip.test.fail", test_error_handler)?;
	let engine = ScriptEngine::from_registry(registry)?;

	// -- Exec
	let lua = engine.lua();
	let result = lua.load("return aip.test.fail({data='boom'})").eval::<mlua::Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("TEST_ERROR"));
	assert!(err_msg.contains("forced test error"));

	Ok(())
}

// endregion: --- Sync handler error conversion

// region:    --- Async invocation

#[tokio::test]
async fn test_script_engine_async_invocation() -> Result<()> {
	// -- Setup & Fixtures
	let mut registry = AipRegistry::default();
	registry.register_async("aip.test.echo_async", test_async_handler)?;
	let engine = ScriptEngine::from_registry(registry)?;

	// -- Exec
	let value: Value = engine
		.lua()
		.load("return aip.test.echo_async({data='async test'})")
		.call_async(())
		.await?;

	// -- Check
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "async test");

	Ok(())
}

// endregion: --- Async invocation

// region:    --- Async handler error conversion

#[tokio::test]
async fn test_script_engine_async_error_conversion() -> Result<()> {
	// -- Setup & Fixtures
	async fn async_error_handler(_: TestParams) -> core::result::Result<TestResponse, AipApiError> {
		Err(AipApiError {
			code: "ASYNC_ERROR".into(),
			message: "forced async error".into(),
			details: None,
			cause: None,
		})
	}

	let mut registry = AipRegistry::default();
	registry.register_async("aip.test.fail_async", async_error_handler)?;
	let engine = ScriptEngine::from_registry(registry)?;

	// -- Exec
	let lua = engine.lua();
	let result = lua
		.load("return aip.test.fail_async({data='boom'})")
		.call_async::<mlua::Value>(())
		.await;

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("ASYNC_ERROR"));
	assert!(err_msg.contains("forced async error"));

	Ok(())
}

// endregion: --- Async handler error conversion

// region:    --- Built-in modules integration

#[test]
fn test_script_engine_new_builtin_modules() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec & Check

	// check aip.json module
	assert!(lua.load("return type(aip.json) == 'table'").eval::<bool>()?);

	assert!(lua.load("return type(aip.json.parse) == 'function'").eval::<bool>()?);

	// check aip.web module
	assert!(lua.load("return type(aip.web) == 'table'").eval::<bool>()?);

	assert!(lua.load("return type(aip.web.get) == 'function'").eval::<bool>()?);

	// check aip.file module
	assert!(lua.load("return type(aip.file) == 'table'").eval::<bool>()?);

	assert!(lua.load("return type(aip.file.read) == 'function'").eval::<bool>()?);

	Ok(())
}

// endregion: --- Built-in modules integration
