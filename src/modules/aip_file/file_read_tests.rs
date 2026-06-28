type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::LuaJsonExt;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn test_read_file_ok() -> Result<()> {
	let tmp = TempDir::new()?;
	let file_path = tmp.path().join("hello.txt");
	std::fs::write(&file_path, "world")?;

	let lua = mlua::Lua::new();

	// Build FileContext using SPath
	let workspace =
		simple_fs::SPath::from_std_path(tmp.path()).map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
	let ctx = FileContext::new(workspace);

	// Register the single handler directly via the registry (for unit test)
	let registry = super::init_registry_with_ctx(ctx)?;

	let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "path": "hello.txt" }))
		.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

	let value = registry.call(lua.clone(), "aip.file.read", params_lua).await?;
	let back = value
		.x_to_json_value()
		.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?
		.ok_or_else(|| mlua::Error::RuntimeError("expected JSON value".to_string()))?;

	assert_eq!(back["content"], json!("world"));
	Ok(())
}
