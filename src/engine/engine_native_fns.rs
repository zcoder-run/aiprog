use crate::ScriptEngine;
use mlua::{self, Value};

impl ScriptEngine {
	pub(super) fn init_native_fns(&self) -> mlua::Result<()> {
		// null, NULL, is_null, value_or, ...
		self.init_null_fns()?;

		// is_object, is_list, is_table
		self.init_is_type_fns()?;

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
}
