use crate::Result;
use crate::registry::{AipFnKind, AipHandlerClosure, AipRegistry};
use crate::script::serde_value_to_lua_value;
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Value};

pub struct ScriptEngine {
	lua: Lua,
}

impl core::fmt::Debug for ScriptEngine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ScriptEngine").finish()
	}
}

impl ScriptEngine {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn from_registry(registry: AipRegistry) -> mlua::Result<Self> {
		Self::from_registry_with_lua(Lua::new(), registry)
	}

	pub fn from_registry_with_lua(lua: Lua, registry: AipRegistry) -> mlua::Result<Self> {
		for entry in registry.entries {
			let func = match entry.kind {
				AipFnKind::Sync => {
					let handler = if let AipHandlerClosure::Sync(handler) = entry.handler {
						handler
					} else {
						return Err(mlua::Error::RuntimeError(
							"Mismatched handler kind for sync entry".into(),
						));
					};
					lua.create_function(move |lua: &Lua, args: MultiValue| -> mlua::Result<Value> {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						handler(lua, arg)
					})?
				}
				AipFnKind::Async => {
					let handler = if let AipHandlerClosure::Async(handler) = entry.handler {
						handler
					} else {
						return Err(mlua::Error::RuntimeError(
							"Mismatched handler kind for async entry".into(),
						));
					};
					lua.create_async_function(move |lua: Lua, args: MultiValue| {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						let response_fut = handler(&lua, arg);
						async move {
							let response_json = response_fut.await?;
							let response_lua = serde_value_to_lua_value(&lua, response_json).map_err(|e| {
								mlua::Error::RuntimeError(format!("Failed to convert response to Lua: {e}"))
							})?;
							Ok::<Value, mlua::Error>(response_lua)
						}
					})?
				}
			};
			install_function_at_path(&lua, &entry.path, func)?;
		}
		Ok(Self { lua })
	}

	pub fn exec(&self, code: &str) -> Result<serde_json::Value> {
		let value = self.lua.load(code).eval()?;
		let value = self.lua.from_value(value)?;

		Ok(value)
	}

	pub fn lua(&self) -> &Lua {
		&self.lua
	}
}

impl Default for ScriptEngine {
	fn default() -> Self {
		Self { lua: Lua::new() }
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

// region:    --- Tests

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

// endregion: --- Tests
