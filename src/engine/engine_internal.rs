use crate::AipHandlerClosure;
use crate::LuaJsonExt;
use crate::{AipFnKind, AipRegistry};
use crate::{Result, ScriptEngine};
use mlua::{Function, Lua, MultiValue, Value};

impl ScriptEngine {
	pub(super) fn register(&mut self, registry: AipRegistry) -> Result<()> {
		let lua = &self.lua;
		self.registered_fns = registry.list_registered_fns();

		for entry in registry.entries {
			let func = match entry.kind {
				AipFnKind::Sync => {
					let handler = if let AipHandlerClosure::Sync(handler) = entry.handler {
						handler
					} else {
						return Err("Mismatched handler kind for sync entry".into());
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
						return Err("Mismatched handler kind for async entry".into());
					};
					let fn_path = entry.path.clone();

					lua.create_async_function(move |lua: Lua, args: MultiValue| {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						let response_fut = handler(&lua, arg);
						let fn_path = fn_path.clone();
						async move {
							let response_json = response_fut.await?;
							let response_lua = mlua::Value::x_from_json_value(&lua, response_json).map_err(|e| {
								mlua::Error::RuntimeError(format!("{fn_path} - Failed to convert response to Lua: {e}"))
							})?;
							Ok::<Value, mlua::Error>(response_lua)
						}
					})?
				}
			};
			install_function_at_path(lua, &entry.path, func)?;
		}

		Ok(())
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
