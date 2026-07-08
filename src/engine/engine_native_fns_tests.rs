use super::*;
use crate::lua_exts::LuaExt;
use mlua::Value;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// region:    --- Built-in modules integration

#[test]
fn test_engine_native_fns_builtin_modules() -> Result<()> {
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

	// check merge globals
	assert!(lua.load("return type(merge) == 'function'").eval::<bool>()?);
	assert!(lua.load("return type(merge_deep) == 'function'").eval::<bool>()?);

	Ok(())
}

// endregion: --- Built-in modules integration

// region:    --- merge tests

#[test]
fn test_engine_native_fns_merge_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = 1}
		local s = {b = 2}
		return merge(t, s)
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);
	assert_eq!(value.x_get_i64("b").ok_or("expected i64 at key 'b'")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_nil_null_skip() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = 1}
		return merge(t, nil, null, {b = 2})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);
	assert_eq!(value.x_get_i64("b").ok_or("expected i64 at key 'b'")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_no_sources() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = 1}
		return merge(t)
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_target_not_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		return merge("not a table", {b = 2})
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("target must be a table"));

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_source_not_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		local t = {a = 1}
		return merge(t, "not a table")
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("Cannot merge a non table type"));

	Ok(())
}

// endregion: --- merge tests

// region:    --- merge_deep tests

#[test]
fn test_engine_native_fns_merge_deep_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = {x = 1}}
		local s = {a = {y = 2}}
		return merge_deep(t, s)
	"#).eval()?;

	// -- Check
	let table = value.as_table().ok_or("Expected table")?;
	let a_table = table.get::<mlua::Table>("a")?;
	assert_eq!(a_table.get::<i64>("x")?, 1);
	assert_eq!(a_table.get::<i64>("y")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_nested() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = {x = {deep = 1}}}
		local s = {a = {x = {other = 2}, y = 3}}
		return merge_deep(t, s)
	"#).eval()?;

	// -- Check
	let table = value.as_table().ok_or("Expected table")?;
	let a_table = table.get::<mlua::Table>("a")?;
	assert_eq!(a_table.get::<i64>("y")?, 3);
	let x_table = a_table.get::<mlua::Table>("x")?;
	assert_eq!(x_table.get::<i64>("deep")?, 1);
	assert_eq!(x_table.get::<i64>("other")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_nil_null_skip() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = 1}
		return merge_deep(t, nil, null, {b = 2})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);
	assert_eq!(value.x_get_i64("b").ok_or("expected i64 at key 'b'")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_no_sources() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		local t = {a = 1}
		return merge_deep(t)
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_target_not_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		return merge_deep("not a table", {b = 2})
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("target must be a table"));

	// -- also nil then non-table
	let result = lua.load(r#"
		return merge_deep(nil, "not a table")
	"#).eval::<Value>();

	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("target must be a table"));

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_source_not_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		local t = {a = 1}
		return merge_deep(t, "not a table")
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("Cannot deep merge a non table type"));

	Ok(())
}

// endregion: --- merge_deep tests

// region:    --- merge nil handling tests

#[test]
fn test_engine_native_fns_merge_all_nil() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge(nil, nil)
	"#).eval()?;

	// -- Check
	assert!(value.is_nil());

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_nil_first_with_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge(nil, {a=1})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_nil_null_mixed() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge(nil, null, {a=1}, {b=2})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);
	assert_eq!(value.x_get_i64("b").ok_or("expected i64 at key 'b'")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_nil_first_non_table_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		return merge(nil, "not a table")
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("target must be a table"));

	Ok(())
}

// endregion: --- merge nil handling tests

// region:    --- merge_deep nil handling tests

#[test]
fn test_engine_native_fns_merge_deep_all_nil() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge_deep(nil, nil)
	"#).eval()?;

	// -- Check
	assert!(value.is_nil());

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_nil_first_with_table() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge_deep(nil, {a=1})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_nil_null_mixed() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let value: Value = lua.load(r#"
		return merge_deep(nil, null, {a=1}, {b=2})
	"#).eval()?;

	// -- Check
	assert_eq!(value.x_get_i64("a").ok_or("expected i64 at key 'a'")?, 1);
	assert_eq!(value.x_get_i64("b").ok_or("expected i64 at key 'b'")?, 2);

	Ok(())
}

#[test]
fn test_engine_native_fns_merge_deep_nil_first_non_table_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = ScriptEngine::new()?;
	let lua = engine.lua();

	// -- Exec
	let result = lua.load(r#"
		return merge_deep(nil, "not a table")
	"#).eval::<Value>();

	// -- Check
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(err_msg.contains("target must be a table"));

	Ok(())
}

// endregion: --- merge_deep nil handling tests
