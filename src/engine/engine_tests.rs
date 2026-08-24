use super::support::LuaEngine;
use crate::impl_lua_serde_traits;
use crate::registry::{HandlerError, HandlerResult};
use crate::{AipIntoLua, AipRegistry, AipRegistryBuilder};
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

#[derive(Debug, Clone, JsonSchema)]
struct AsyncLuaOnlyResponse {
	data: String,
}

impl_lua_serde_traits!(TestParams);
impl_lua_serde_traits!(TestResponse);

impl crate::AipParams for TestParams {}
impl crate::AipOutput for TestResponse {}
impl crate::AipOutput for AsyncLuaOnlyResponse {}

impl AipIntoLua for AsyncLuaOnlyResponse {
	fn into_lua(self, lua: &mlua::Lua) -> crate::Result<Value> {
		let table = lua.create_table()?;
		table.set("data", self.data)?;
		Ok(Value::Table(table))
	}
}

fn test_sync_handler(_call: crate::HandlerCallContext, params: TestParams) -> HandlerResult<TestResponse> {
	Ok(TestResponse { data: params.data })
}

async fn test_async_handler(_call: crate::HandlerCallContext, params: TestParams) -> HandlerResult<TestResponse> {
	Ok(TestResponse { data: params.data })
}

async fn test_async_lua_only_handler(
	_call: crate::HandlerCallContext,
	params: TestParams,
) -> HandlerResult<AsyncLuaOnlyResponse> {
	Ok(AsyncLuaOnlyResponse { data: params.data })
}

fn test_error_handler(_call: crate::HandlerCallContext, _params: TestParams) -> HandlerResult<TestResponse> {
	Err(HandlerError::custom("[TEST_ERROR] forced test error"))
}

// region:    --- Nested table creation

#[test]
fn test_lua_engine_nested_table_creation() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default()
		.register_sync("aip.test.my_func", test_sync_handler)?
		.build();

	// -- Exec
	let engine = LuaEngine::from_registry(registry)?;

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
fn test_lua_engine_existing_compatible_table() -> Result<()> {
	// -- Setup & Fixtures
	let mut engine = LuaEngine::new()?;
	{
		let lua = engine.lua();

		let aip = lua.create_table()?;
		let existing = lua.create_table()?;
		aip.set("existing", existing.clone())?;
		lua.globals().set("aip", aip)?;
	}

	let registry = AipRegistryBuilder::default()
		.register_sync("aip.existing.my_func", test_sync_handler)?
		.build();

	// -- Exec
	engine.register(registry)?;

	// -- Check
	let lua = engine.lua();
	let value: Value = lua.load("return aip.existing.my_func({data='world'})").eval()?;
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "world");

	Ok(())
}

// endregion: --- Existing compatible intermediate table

// region:    --- Intermediate non-table conflict

#[test]
fn test_lua_engine_intermediate_non_table_conflict() -> Result<()> {
	// -- Setup & Fixtures
	let mut engine = LuaEngine::new()?;
	{
		let lua = engine.lua();

		// Set aip.existing to a number instead of table
		lua.globals().set("aip", lua.create_table()?)?;
		let aip_table: mlua::Table = lua.globals().get("aip")?;
		aip_table.set("existing", 42)?;
	}

	let registry = AipRegistryBuilder::default()
		.register_sync("aip.existing.my_func", test_sync_handler)?
		.build();

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
fn test_lua_engine_leaf_conflict() -> Result<()> {
	// -- Setup & Fixtures
	let mut engine = LuaEngine::new()?;
	{
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
	}

	let registry = AipRegistryBuilder::default()
		.register_sync("aip.conflict.my_func", test_sync_handler)?
		.build();

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
fn test_lua_engine_sync_invocation() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default()
		.register_sync("aip.test.echo", test_sync_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

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
fn test_lua_engine_sync_error_conversion() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default()
		.register_sync("aip.test.fail", test_error_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

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
async fn test_lua_engine_async_invocation() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default()
		.register_async("aip.test.echo_async", test_async_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

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
async fn test_lua_engine_async_error_conversion() -> Result<()> {
	// -- Setup & Fixtures
	async fn async_error_handler(_: crate::HandlerCallContext, _: TestParams) -> HandlerResult<TestResponse> {
		Err(HandlerError::custom("[ASYNC_ERROR] forced async error"))
	}

	let registry = AipRegistryBuilder::default()
		.register_async("aip.test.fail_async", async_error_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

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

// region:    --- generate_doc test

#[test]
fn test_generate_doc_content() -> Result<()> {
	let registry = AipRegistryBuilder::default()
		.register_sync("aip.test.echo", test_sync_handler)?
		.register_async("aip.test.echo_async", test_async_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

	let doc = engine.generate_doc()?;

	// Check for expected sections
	assert!(doc.contains("## aip.test.*"));
	assert!(doc.contains("echo(params: AipTestEchoParams): AipTestEchoOutput"));
	assert!(doc.contains("echo_async(params: AipTestEchoParams): AipTestEchoOutput"));
	// // Verify TypeScript block and types are present
	assert!(doc.contains("```ts"));
	assert!(doc.contains("### aip.test.* Types"));
	assert!(doc.contains("type AipTestEchoParams ="));

	Ok(())
}

// endregion: --- generate_doc test

// region:    --- Rich error paths

#[tokio::test]
async fn test_lua_engine_exec_invalid_params_rich_message() -> Result<()> {
	// -- Setup & Fixtures
	let engine = LuaEngine::new()?;
	let script = r#"
local res = aip.web.get({ url = 42 })
return res
"#;

	// -- Exec
	let result = engine.exec(script).await;

	// -- Check
	assert!(result.is_err());
	let err = result.unwrap_err();
	let crate::Error::LuaScript(details) = err else {
		return Err("Expected Error::LuaScript".into());
	};
	let msg = details.message();
	assert!(
		msg.contains("aip.web.get"),
		"message should contain handler path: {msg}"
	);
	assert!(msg.contains("'url'"), "message should contain field name: {msg}");
	assert!(msg.contains("string"), "message should contain expected type: {msg}");
	assert!(msg.contains("integer"), "message should contain actual type: {msg}");
	assert!(msg.contains("42"), "message should contain value preview: {msg}");

	Ok(())
}

#[tokio::test]
async fn test_luan_engine_context_free_file_handler_reports_missing_context() -> Result<()> {
	// -- Setup & Fixtures
	let engine = LuaEngine::new_context_free()?;

	// -- Exec
	let result = engine.exec("return aip.file.exists({ path = 'missing.txt' })").await;

	// -- Check
	let error = result.err().ok_or("Should reject a context-dependent file handler")?;
	let crate::Error::LuaScript(details) = error else {
		return Err("Expected Error::LuaScript".into());
	};
	assert!(details.message().contains("Running context does not contain a value of type"));
	assert!(details.message().contains("DirContext"));

	Ok(())
}

#[tokio::test]
async fn test_lua_engine_exec_lua_script_error_details() -> Result<()> {
	// -- Setup & Fixtures
	let engine = LuaEngine::from_registry(AipRegistry::from_empty())?;
	let script = "local a = 1\nlocal b = 2\nerror('boom')\nreturn a + b";

	// -- Exec
	let result = engine.exec(script).await;

	// -- Check
	assert!(result.is_err());
	let err = result.unwrap_err();
	let crate::Error::LuaScript(details) = err else {
		return Err("Expected Error::LuaScript".into());
	};
	assert_eq!(details.line_number(), Some(3));
	let surround = details.surround_code().ok_or("Expected surround_code")?;
	assert!(surround.contains("error('boom')"));
	assert!(surround.contains("> 3 |"));
	assert!(details.message().contains("boom"));
	assert!(!details.message().contains("stack traceback:"));

	Ok(())
}

// endregion: --- Rich error paths

// region:    --- Async Lua-only output

#[tokio::test]
async fn test_lua_engine_async_lua_only_output() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default()
		.register_async("aip.test.lua_only", test_async_lua_only_handler)?
		.build();
	let engine = LuaEngine::from_registry(registry)?;

	// -- Exec
	let value: Value = engine
		.lua()
		.load("return aip.test.lua_only({data='lua native'})")
		.call_async(())
		.await?;

	// -- Check
	let table = value.as_table().ok_or("Expected table")?;
	assert_eq!(table.get::<String>("data")?, "lua native");

	Ok(())
}

// endregion: --- Async Lua-only output

// region:    --- Standard libraries

#[test]
fn test_lua_engine_stdlib_configuration() -> Result<()> {
	// -- Setup & Fixtures
	let engine = LuaEngine::new()?;
	let lua = engine.lua();

	// -- Check enabled libraries
	assert!(lua.globals().get::<mlua::Table>("string").is_ok());
	assert!(lua.globals().get::<mlua::Table>("math").is_ok());
	assert!(lua.globals().get::<mlua::Table>("table").is_ok());
	assert!(lua.globals().get::<mlua::Table>("utf8").is_ok());

	// -- Check disabled libraries
	assert_eq!(lua.globals().get::<mlua::Value>("os")?, Value::Nil);
	assert_eq!(lua.globals().get::<mlua::Value>("package")?, Value::Nil);
	assert_eq!(lua.globals().get::<mlua::Value>("debug")?, Value::Nil);
	assert_eq!(lua.globals().get::<mlua::Value>("coroutine")?, Value::Nil);
	assert_eq!(lua.globals().get::<mlua::Value>("io")?, Value::Nil);

	// -- Check standard library functions work
	let str_val: String = lua.load("return string.sub('hello world', 1, 5)").eval()?;
	assert_eq!(str_val, "hello");

	let math_val: i64 = lua.load("return math.floor(42.7)").eval()?;
	assert_eq!(math_val, 42);

	let tbl_val: String = lua.load("return table.concat({'a', 'b', 'c'}, '-')").eval()?;
	assert_eq!(tbl_val, "a-b-c");

	let utf8_val: usize = lua.load("return utf8.len('hello')").eval()?;
	assert_eq!(utf8_val, 5);

	Ok(())
}

// endregion: --- Standard libraries
