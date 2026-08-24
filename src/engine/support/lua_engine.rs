//! Home of the concrete internal `LuaEngine` state and engine-only support APIs.
//!
//! Content:
//!
//! - The `LuaEngine` struct fields and their crate-private visibility boundary.
//! - The private Lua accessor used by the engine implementation files.
//!
//! The public `LuaEngine` constructors, execution APIs, and documentation APIs stay in the
//! top-level `src/engine/` files.

use crate::AipRegisteredFn;
use crate::{LuaErrorDetails, LuaJsonExt as _, Result};
use mlua::Lua;

pub struct LuaEngine {
	pub(in crate::engine) lua: Lua,
	pub(in crate::engine) registered_fns: Vec<AipRegisteredFn>,
}

impl core::fmt::Debug for LuaEngine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ScriptEngine").finish()
	}
}

/// Exec
impl LuaEngine {
	/// Exec a script with this engine, and return the mlua::Value
	pub async fn exec(&self, script: &str) -> Result<serde_json::Value> {
		let lua_value = self.exec_raw(script).await?;
		let value = lua_value.x_to_json_value()?.unwrap_or_default();

		Ok(value)
	}

	/// Exec a script with this engine, and return the mlua::Value
	pub async fn exec_raw(&self, script: &str) -> Result<mlua::Value> {
		let lua = self.lua();

		let func = lua
			.load(script)
			.set_name("=script")
			.into_function()
			.map_err(|err| LuaErrorDetails::from_lua_error(&err, script))?;
		let value: mlua::Value = func
			.call_async(())
			.await
			.map_err(|err| LuaErrorDetails::from_lua_error(&err, script))?;
		Ok(value)
	}
}

// region:    --- Support

impl LuaEngine {
	pub(in crate::engine) fn lua(&self) -> &Lua {
		&self.lua
	}
}

// endregion: --- Support
