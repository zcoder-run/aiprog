use crate::Result;
use crate::registry::{AipFnKind, AipHandlerClosure, AipRegistry};
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Value};

pub struct ScriptEngine {
	pub(super) lua: Lua,
}

impl core::fmt::Debug for ScriptEngine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ScriptEngine").finish()
	}
}

impl ScriptEngine {
	/// Create a new script engine with default settings
	pub fn new() -> Result<Self> {
		let engine = Self { lua: Lua::new() };
		engine.init_native_is()?;
		let registry = crate::script::modules::init_registry()?;
		engine.register(registry)?;
		Ok(engine)
	}

	pub fn from_registry(registry: AipRegistry) -> Result<Self> {
		let engine = Self { lua: Lua::new() };
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

/// Install a Lua function at a dotted path, creating intermediate tables as needed.
fn install_function_at_path(lua: &Lua, path: &str, func: Function) -> mlua::Result<()> {
	let segments: Vec<&str> = path.split('.').collect();
	if segments.is_empty() {
		return Err(mlua::Error::RuntimeError(
			"Invalid empty path for function installation".into(),
		));
	}
	let (leaf, ancestors) = segments.split_last().unwrap();
	let globals = lua.globals();
	let mut current = globals;
	for &seg in ancestors {
		let next: Value = current.get(seg)?;
		if next.is_nil() {
			let table = lua.create_table()?;
			current.set(seg, table.clone())?;
			current = table;
		} else if let Value::Table(t) = next {
			current = t;
		} else {
			return Err(mlua::Error::RuntimeError(format!(
				"Path segment '{}' exists but is not a table",
				seg
			)));
		}
	}
	// Reject targeted leaf conflicts by default
	if let Ok(existing) = current.get::<Value>(*leaf)
		&& !existing.is_nil()
	{
		return Err(mlua::Error::RuntimeError(format!(
			"Function already exists at leaf '{}' in path '{}'",
			leaf, path
		)));
	}
	current.set(*leaf, func)?;
	Ok(())
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
