use crate::registry::{AipFnKind, AipHandlerClosure, AipRegistry};
use crate::script::LuaJsonExt;
use crate::{Result, ScriptEngine};
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Value};

impl ScriptEngine {
	pub(super) fn register(&self, registry: AipRegistry) -> Result<()> {
		let lua = &self.lua;

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

					lua.create_async_function(move |lua: Lua, args: MultiValue| {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						let response_fut = handler(&lua, arg);
						async move {
							let response_json = response_fut.await?;
							let response_lua = mlua::Value::x_from_json_value(&lua, response_json).map_err(|e| {
								mlua::Error::RuntimeError(format!("Failed to convert response to Lua: {e}"))
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

	pub(super) fn init_native_is(&self) -> mlua::Result<()> {
		// region:    --- init_null

		let globals = self.lua.globals();
		globals.set("null", Value::NULL)?;
		globals.set("Null", Value::NULL)?;
		globals.set("NULL", Value::NULL)?;

		// is_null(x) -> boolean
		globals.set(
			"is_null",
			self.lua
				.create_function(|_, v: Value| Ok(matches!(v, Value::Nil) || v == Value::NULL))?,
		)?;

		// nil_if_null(x) -> x or nil
		globals.set(
			"nil_if_null",
			self.lua.create_function(|_, v: Value| {
				if matches!(v, Value::Nil) || v == Value::NULL {
					Ok(Value::Nil)
				} else {
					Ok(v)
				}
			})?,
		)?;

		// value_or(value, alt) -> value or alt
		globals.set(
			"value_or",
			self.lua.create_function(|_, (v, alt): (Value, Value)| {
				if matches!(v, Value::Nil) || v == Value::NULL {
					Ok(alt)
				} else {
					Ok(v)
				}
			})?,
		)?;

		// is_not_null(x) -> boolean
		globals.set(
			"is_not_null",
			self.lua
				.create_function(|_, v: Value| Ok(!(matches!(v, Value::Nil) || v == Value::NULL)))?,
		)?;

		// is_table(x) -> boolean
		globals.set(
			"is_table",
			self.lua.create_function(|_, v: Value| {
				if matches!(v, Value::Nil) || v == Value::NULL {
					return Ok(false);
				}
				Ok(matches!(v, Value::Table(_)))
			})?,
		)?;

		// is_list(x) -> boolean
		globals.set(
			"is_list",
			self.lua.create_function(|_, v: Value| {
				if matches!(v, Value::Nil) || v == Value::NULL {
					return Ok(false);
				}
				if let Value::Table(t) = v {
					let val = t.raw_get(1)?;
					Ok(!matches!(val, Value::Nil))
				} else {
					Ok(false)
				}
			})?,
		)?;

		// is_object(x) -> boolean
		globals.set(
			"is_object",
			self.lua.create_function(|_, v: Value| {
				if matches!(v, Value::Nil) || v == Value::NULL {
					return Ok(false);
				}
				if let Value::Table(t) = v {
					let val = t.raw_get(1)?;
					Ok(matches!(val, Value::Nil))
				} else {
					Ok(false)
				}
			})?,
		)?;

		// endregion: --- init_null

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
