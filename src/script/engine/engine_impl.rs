use crate::AipRegistry;
use crate::Result;
use crate::script::AipRegisteredFn;
use mlua::{Lua, LuaSerdeExt};

pub struct ScriptEngine {
	pub(super) lua: Lua,
	pub(super) registered_fns: Vec<AipRegisteredFn>,
}

impl core::fmt::Debug for ScriptEngine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ScriptEngine").finish()
	}
}

impl ScriptEngine {
	/// Create a new script engine with default settings
	pub fn new() -> Result<Self> {
		let mut engine = Self {
			lua: Lua::new(),
			registered_fns: Vec::new(),
		};
		engine.init_native_is()?;
		let registry = crate::script::modules::init_registry()?;
		engine.register(registry)?;
		Ok(engine)
	}

	pub fn from_registry(registry: AipRegistry) -> Result<Self> {
		let mut engine = Self {
			lua: Lua::new(),
			registered_fns: Vec::new(),
		};
		engine.init_native_is()?;
		engine.register(registry)?;
		Ok(engine)
	}

	pub fn exec(&self, code: &str) -> Result<serde_json::Value> {
		let value = self.lua.load(code).eval()?;
		let value = self.lua.from_value(value)?;

		Ok(value)
	}

	pub fn lua(&self) -> &Lua {
		&self.lua
	}

	// -- set_value_at_path
	/// Install any value at a dotted Lua path, creating intermediate tables as needed.
	pub fn set_value_at_path(&self, path: &str, value: mlua::Value) -> mlua::Result<()> {
		install_value_at_path(&self.lua, path, value)
	}
}

/// Install a Lua value at a dotted path, creating intermediate tables as needed.
pub(crate) fn install_value_at_path(lua: &Lua, path: &str, value: mlua::Value) -> mlua::Result<()> {
	let segments: Vec<_> = path.split('.').collect();
	if segments.is_empty() {
		return Err(mlua::Error::RuntimeError(
			"Invalid empty path for value installation".into(),
		));
	}
	let (leaf, ancestors) = segments.split_last().unwrap();
	let globals = lua.globals();
	let mut current = globals;
	for &seg in ancestors {
		let next: mlua::Value = current.get(seg)?;
		if next.is_nil() {
			let table = lua.create_table()?;
			current.set(seg, table.clone())?;
			current = table;
		} else if let mlua::Value::Table(t) = next {
			current = t;
		} else {
			return Err(mlua::Error::RuntimeError(format!(
				"Path segment '{}' exists but is not a table",
				seg
			)));
		}
	}
	current.set(*leaf, value)?;
	Ok(())
}
