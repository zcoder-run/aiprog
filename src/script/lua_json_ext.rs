use crate::{Error, Result, script::LuaExt};
use mlua::{Lua, LuaSerdeExt as _, Table, Value};

/// Lua JSON conversion extension trait for Lua values.
pub trait LuaJsonExt: LuaExt {
	/// Convert a serde_json::Value into a mlua::Value.
	fn from_json_value(lua: &Lua, val: serde_json::Value) -> Result<Value>;

	/// Convert an iterable of JSON values into a Vec<mlua::Value>.
	fn from_json_values<I>(lua: &Lua, values: I) -> Result<Vec<Value>>
	where
		I: IntoIterator<Item = serde_json::Value>;

	/// Convenient function to take lua value to serde value
	///
	/// NOTE: The app should use this one rather to call serde_json::to_value directly
	///       This way we can normalize the behavior and error and such.
	///
	/// Custom logic:
	/// - Converts Lua tables either as arrays (when contiguous 1..n integer keys without gaps) or objects (stringified keys).
	fn to_json_value(&self) -> Result<serde_json::Value>;

	/// If this Lua value is a table/list, convert it into a Vec<serde_json::Value>.
	fn to_json_values(&self) -> Result<Vec<serde_json::Value>>;
}

impl LuaJsonExt for Value {
	fn from_json_value(lua: &Lua, val: serde_json::Value) -> Result<Value> {
		match val {
			serde_json::Value::Null => Ok(Value::NULL),
			other => Ok(lua.to_value(&other)?),
		}
	}

	fn from_json_values<I>(lua: &Lua, values: I) -> Result<Vec<Value>>
	where
		I: IntoIterator<Item = serde_json::Value>,
	{
		values.into_iter().map(|v| Self::from_json_value(lua, v)).collect()
	}

	fn to_json_value(&self) -> Result<serde_json::Value> {
		fn number_from_f64(v: f64) -> Result<serde_json::Number> {
			serde_json::Number::from_f64(v)
				.ok_or_else(|| Error::custom("Cannot convert non-finite Lua number to JSON (NaN or Infinity)"))
		}

		fn convert_table(table: mlua::Table) -> Result<serde_json::Value> {
			// Try to treat as an array (1..n contiguous integer keys, no gaps)
			let mut max_idx: usize = 0;
			let mut numeric_only = true;

			for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
				let (k, _) = pair?;
				match k {
					Value::Integer(i) if i > 0 => {
						let i = i as usize;
						if i > max_idx {
							max_idx = i;
						}
					}
					Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => {
						let i = n as usize;
						if i > max_idx {
							max_idx = i;
						}
					}
					_ => {
						numeric_only = false;
						break;
					}
				}
			}

			if numeric_only {
				let mut vec: Vec<Option<serde_json::Value>> = vec![None; max_idx];
				for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
					let (k, v) = pair?;
					let idx_opt = match k {
						Value::Integer(i) if i > 0 => Some(i as usize),
						Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => Some(n as usize),
						_ => None,
					};
					if let Some(idx) = idx_opt {
						if idx == 0 || idx > max_idx {
							numeric_only = false;
							break;
						}
						vec[idx - 1] = Some(v.to_json_value()?);
					} else {
						numeric_only = false;
						break;
					}
				}

				if numeric_only && vec.iter().all(|o| o.is_some()) {
					let arr = vec.into_iter().flatten().collect();
					return Ok(serde_json::Value::Array(arr));
				}
			}

			// Fallback: treat as object with stringified keys
			let mut map = serde_json::Map::new();
			for pair in table.pairs::<mlua::Value, mlua::Value>() {
				let (k, v) = pair?;
				let key = match k {
					Value::String(s) => s.to_str()?.to_string(),
					Value::Integer(i) => i.to_string(),
					Value::Number(n) => n.to_string(),
					Value::Boolean(b) => b.to_string(),
					other => {
						return Err(Error::custom(format!(
							"Unsupported Lua table key type '{}' for JSON object",
							other.type_name()
						)));
					}
				};
				map.insert(key, v.to_json_value()?);
			}
			Ok(serde_json::Value::Object(map))
		}

		let lua_value = self.clone();
		match lua_value {
			Value::Nil => Ok(serde_json::Value::Null),
			Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
			Value::Integer(i) => Ok(serde_json::Value::Number(serde_json::Number::from(i))),
			Value::Number(n) => Ok(serde_json::Value::Number(number_from_f64(n)?)),
			Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
			Value::Table(t) => convert_table(t),
			Value::LightUserData(ldata) => {
				if Value::LightUserData(ldata) == Value::NULL {
					Ok(serde_json::Value::Null)
				} else {
					// for now, still null
					Ok(serde_json::Value::Null)
				}
			}
			Value::Function(_) | Value::Thread(_) | Value::UserData(_) => Err(Error::custom(
				"Cannot serialize Lua value to JSON: unsupported type (Function/LigthUserData/UserData)",
			)),
			Value::Error(_) => Err(Error::custom(
				"Cannot serialize Lua value to JSON: unsupported type (error)",
			)),
			Value::Other(_) => Err(Error::custom(
				"Cannot serialize Lua value to JSON: unsupported type (other)",
			)),
		}
	}

	fn to_json_values(&self) -> Result<Vec<serde_json::Value>> {
		let vals = self.x_as_list();
		let val = self.clone();
		let table = match val {
			Value::Table(t) => t,
			other => {
				return Err(Error::custom(format!(
					"Lua Value is not a List. Expected a Lua table (list) as the second argument, but got {}",
					other.type_name()
				)));
			}
		};
		let iter = table.sequence_values::<Value>();
		let json_values: Vec<serde_json::Value> = iter
			.into_iter()
			.map(|v| v?.to_json_value())
			.collect::<Result<Vec<_>>>()
			.map_err(|e| Error::custom(format!("A mlua Value cannot be serialize to Json.\nCause: {e}",)))?;
		Ok(json_values)
	}
}

impl LuaJsonExt for Table {
	fn from_json_value(lua: &Lua, val: serde_json::Value) -> Result<Value> {
		Value::from_json_value(lua, val)
	}

	fn from_json_values<I>(lua: &Lua, values: I) -> Result<Vec<Value>>
	where
		I: IntoIterator<Item = serde_json::Value>,
	{
		Value::from_json_values(lua, values)
	}

	fn to_json_value(&self) -> Result<serde_json::Value> {
		let val: Value = Value::Table(self.clone());
		val.to_json_value()
	}

	fn to_json_values(&self) -> Result<Vec<serde_json::Value>> {
		let val: Value = Value::Table(self.clone());
		val.to_json_values()
	}
}
