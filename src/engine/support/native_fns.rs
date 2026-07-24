//! Home of the native Lua helper initialization.
//!
//! Content:
//!
//! - The `null` family of helpers, `null`, `is_null`, `nil_if_null`, `value_or`, `is_not_null`.
//! - The type predicates, `is_table`, `is_list`, `is_object`.
//! - The table merge helpers, `merge` and `merge_deep`, with their deep-merge support function.

use crate::engine::support::ScriptEngine;
use mlua::{self, Value, Variadic};

impl ScriptEngine {
	pub(in crate::engine) fn init_native_fns(&self) -> mlua::Result<()> {
		// null, NULL, is_null, value_or, ...
		self.init_null_fns()?;

		// is_object, is_list, is_table
		self.init_is_type_fns()?;

		// merge, merge_deep
		self.init_merge_fns()?;

		Ok(())
	}
}

/// Private implemenations
impl ScriptEngine {
	fn init_null_fns(&self) -> mlua::Result<()> {
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

		// value_or(value, alt, ...) -> first non-null value or nil
		globals.set(
			"value_or",
			self.lua.create_function(|_, values: Variadic<Value>| {
				for v in values {
					if !matches!(v, Value::Nil) && v != Value::NULL {
						return Ok(v);
					}
				}
				Ok(Value::Nil)
			})?,
		)?;

		// is_not_null(x) -> boolean
		globals.set(
			"is_not_null",
			self.lua
				.create_function(|_, v: Value| Ok(!(matches!(v, Value::Nil) || v == Value::NULL)))?,
		)?;

		Ok(())
	}

	fn init_is_type_fns(&self) -> mlua::Result<()> {
		let globals = self.lua.globals();

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

		Ok(())
	}

	fn init_merge_fns(&self) -> mlua::Result<()> {
		let globals = self.lua.globals();

		// merge(target, ...sources) - nil first argument is allowed
		globals.set(
			"merge",
			self.lua.create_function(|_lua, values: mlua::Variadic<mlua::Value>| {
				if values.is_empty() {
					return Err(mlua::Error::RuntimeError(
						"merge requires at least a target table".into(),
					));
				}
				let mut iter = values.into_iter();
				let mut target_table: Option<mlua::Table> = None;
				let mut target_value: Option<mlua::Value> = None;

				// find the first non-nil/non-null table as the target
				for arg in iter.by_ref() {
					if arg.is_nil() || arg == Value::NULL {
						continue;
					}
					if let Value::Table(ref t) = arg {
						target_table = Some(t.clone());
						target_value = Some(arg);
						break;
					} else {
						return Err(mlua::Error::RuntimeError("merge: target must be a table".into()));
					}
				}

				let target_table = match target_table {
					Some(t) => t,
					None => return Ok(Value::Nil),
				};

				// merge remaining sources
				for src in iter {
					if src.is_nil() || src == Value::NULL {
						continue;
					}
					if let Value::Table(ref src_table) = src {
						for pair in src_table.pairs::<Value, Value>() {
							let (key, val) = pair?;
							target_table.set(key, val)?;
						}
					} else {
						return Err(mlua::Error::RuntimeError("Cannot merge a non table type".into()));
					}
				}
				Ok(target_value.unwrap())
			})?,
		)?;

		// merge_deep(target, ...sources) - nil first argument is allowed
		globals.set(
			"merge_deep",
			self.lua.create_function(|_lua, values: mlua::Variadic<mlua::Value>| {
				if values.is_empty() {
					return Err(mlua::Error::RuntimeError(
						"merge_deep requires at least a target table".into(),
					));
				}
				let mut iter = values.into_iter();
				let mut target_table: Option<mlua::Table> = None;
				let mut target_value: Option<mlua::Value> = None;

				// find the first non-nil/non-null table as the target
				for arg in iter.by_ref() {
					if arg.is_nil() || arg == Value::NULL {
						continue;
					}
					if let Value::Table(ref t) = arg {
						target_table = Some(t.clone());
						target_value = Some(arg);
						break;
					} else {
						return Err(mlua::Error::RuntimeError("merge_deep: target must be a table".into()));
					}
				}

				let target_table = match target_table {
					Some(t) => t,
					None => return Ok(Value::Nil),
				};

				// merge remaining sources
				for src in iter {
					if src.is_nil() || src == Value::NULL {
						continue;
					}
					if let Value::Table(ref src_table) = src {
						merge_tables_deep(&target_table, src_table)?;
					} else {
						return Err(mlua::Error::RuntimeError("Cannot deep merge a non table type".into()));
					}
				}
				Ok(target_value.unwrap())
			})?,
		)?;

		Ok(())
	}
}

// region:    --- Support

fn merge_tables_deep(target: &mlua::Table, source: &mlua::Table) -> mlua::Result<()> {
	for pair in source.pairs::<mlua::Value, mlua::Value>() {
		let (key, src_val) = pair?;
		let tgt_val: mlua::Value = target.get(key.clone())?;
		if let mlua::Value::Table(ref tgt_table) = tgt_val
			&& let mlua::Value::Table(ref src_table) = src_val
		{
			merge_tables_deep(tgt_table, src_table)?;
		} else {
			target.set(key, src_val)?;
		}
	}
	Ok(())
}

// endregion: --- Support
