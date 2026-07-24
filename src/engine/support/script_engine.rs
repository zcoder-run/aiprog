//! Home of the concrete internal `ScriptEngine` state and engine-only support APIs.
//!
//! Content:
//!
//! - The `ScriptEngine` struct fields and their crate-private visibility boundary.
//! - The private Lua accessor used by the engine implementation files.
//!
//! The public `ScriptEngine` constructors, execution APIs, and documentation APIs stay in the
//! top-level `src/engine/` files.

use crate::AipRegisteredFn;
use crate::{AipRegistry, LuaErrorDetails, LuaJsonExt as _, Result};
use mlua::{IntoLua, Lua};

use super::install_value_at_path;

pub struct ScriptEngine {
	pub(in crate::engine) lua: Lua,
	pub(in crate::engine) registered_fns: Vec<AipRegisteredFn>,
}

impl core::fmt::Debug for ScriptEngine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ScriptEngine").finish()
	}
}

/// Constructors
impl ScriptEngine {
	/// Create a new script engine with default settings
	#[allow(dead_code)]
	pub fn new() -> Result<Self> {
		Self::new_context_free()
	}

	/// Create a script engine for APIs that do not require execution context.
	///
	/// Context-dependent handlers must run through [`EngineTemplate`](super::EngineTemplate)
	/// with a caller-supplied [`RunningContext`](crate::RunningContext).
	#[allow(dead_code)]
	pub fn new_context_free() -> Result<Self> {
		let mut engine = Self {
			lua: Lua::new(),
			registered_fns: Vec::new(),
		};
		engine.init_native_fns()?;
		let registry = crate::modules::init_registry()?;
		engine.register(registry)?;
		Ok(engine)
	}

	#[allow(dead_code)]
	pub fn from_registry(registry: AipRegistry) -> Result<Self> {
		Self::from_context_free_registry(registry)
	}

	/// Create a script engine for a registry whose handlers do not require execution context.
	///
	/// Context-dependent handlers must run through [`EngineTemplate`](super::EngineTemplate)
	/// with a caller-supplied [`RunningContext`](crate::RunningContext).
	pub fn from_context_free_registry(registry: AipRegistry) -> Result<Self> {
		let mut engine = Self {
			lua: Lua::new(),
			registered_fns: Vec::new(),
		};
		engine.init_native_fns()?;
		engine.register(registry)?;
		Ok(engine)
	}
}

/// Exec
impl ScriptEngine {
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

/// Others public
impl ScriptEngine {
	/// Install any value at a dotted Lua path, creating intermediate tables as needed.
	pub fn set_value_at_path(&self, path: &str, value: impl IntoLua) -> mlua::Result<()> {
		let value = IntoLua::into_lua(value, self.lua())?;
		install_value_at_path(self.lua(), path, value)
	}
}

// region:    --- Support

impl ScriptEngine {
	pub(in crate::engine) fn lua(&self) -> &Lua {
		&self.lua
	}
}

// endregion: --- Support
