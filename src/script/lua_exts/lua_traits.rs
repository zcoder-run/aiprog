use crate::Result;
use crate::script::HandlerError;
use crate::script::LuaExt;
// Re-export for convenience
pub use mlua::Value as LuaValue;
use mlua::{Lua, Value};
use std::collections::HashMap;

pub trait AipFromLua: Sized {
	fn from_lua(lua: &Lua, value: Value) -> Result<Self>;
}

pub trait AipToLua {
	fn to_lua(self, lua: &Lua) -> Result<Value>;
}

// region:    --- FromLua implementations

impl AipFromLua for Value {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		Ok(value)
	}
}

impl AipFromLua for String {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		value
			.x_as_lua_str()
			.map(|s| s.to_string())
			.ok_or_else(|| "expected string".into())
	}
}

impl AipFromLua for i64 {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		Ok(value
			.x_as_i64()
			.ok_or_else(|| HandlerError::new("expected integer".to_string()))?)
	}
}

impl AipFromLua for f64 {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		Ok(value.x_as_f64().ok_or_else(|| HandlerError::new("expected number".to_string()))?)
	}
}

impl AipFromLua for bool {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		Ok(value
			.as_boolean()
			.ok_or_else(|| HandlerError::new("expected boolean".to_string()))?)
	}
}

impl<T: AipFromLua> AipFromLua for Option<T> {
	fn from_lua(lua: &Lua, value: Value) -> Result<Self> {
		if value.is_nil() || value.x_is_null() {
			Ok(None)
		} else {
			Ok(Some(T::from_lua(lua, value)?))
		}
	}
}

impl<T: AipFromLua> AipFromLua for Vec<T> {
	fn from_lua(lua: &Lua, value: Value) -> Result<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| HandlerError::new("expected table".to_string()))?;
		let mut vec = Vec::new();
		for val in table.sequence_values::<Value>() {
			let val = val.map_err(|e| HandlerError::new(e.to_string()))?;
			vec.push(T::from_lua(lua, val)?);
		}
		Ok(vec)
	}
}

impl<T: AipFromLua> AipFromLua for HashMap<String, T> {
	fn from_lua(lua: &Lua, value: Value) -> Result<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| HandlerError::new("expected table".to_string()))?;
		let mut map = HashMap::new();
		for pair in table.pairs::<Value, Value>() {
			let (key, val) = pair.map_err(|e| HandlerError::new(e.to_string()))?;
			let key_str = match key {
				Value::String(s) => s.to_str().map_err(|e| HandlerError::new(e.to_string()))?.to_string(),
				Value::Integer(i) => i.to_string(),
				Value::Number(n) => n.to_string(),
				Value::Boolean(b) => b.to_string(),
				other => {
					return Err(crate::Error::Handler(HandlerError::new(format!(
						"unsupported Lua table key type: {}",
						other.type_name()
					))));
				}
			};
			map.insert(key_str, T::from_lua(lua, val)?);
		}
		Ok(map)
	}
}

impl AipFromLua for serde_json::Value {
	fn from_lua(_lua: &Lua, value: Value) -> Result<Self> {
		Ok(crate::script::LuaJsonExt::x_to_json_value(&value)
			.map_err(|e| HandlerError::new(e.to_string()))?
			.ok_or_else(|| HandlerError::new("cannot convert Lua nil to JSON value".to_string()))?)
	}
}

// region:    --- Convenience macro for test types (or any type using serde)

/// Generates `FromLua` and `ToLua` implementations for a type that implements
/// `serde::Serialize` and `serde::de::DeserializeOwned`, piggy‑backing on the
/// existing `serde_json::Value` ↔ `mlua::Value` conversion.
#[macro_export]
macro_rules! impl_lua_serde_traits {
	($ty:path) => {
		impl $crate::script::AipFromLua for $ty {
			fn from_lua(
				_lua: &mlua::Lua,
				value: mlua::Value,
	) -> $crate::Result<Self> {
				let serde_value = $crate::script::LuaJsonExt::x_to_json_value(&value).map_err(|e| {
					$crate::script::HandlerError::new($crate::script::AipApiError::new("INVALID_PARAMS", e.to_string()))
				})?;
				let serde_value = serde_value.ok_or_else(|| {
					$crate::script::HandlerError::new($crate::script::AipApiError::new(
						"INVALID_PARAMS",
						"expected JSON value, got nil".to_string(),
					))
				})?;
		Ok(serde_json::from_value(serde_value).map_err(|e| {
					$crate::script::HandlerError::new($crate::script::AipApiError::new(
						"INVALID_PARAMS",
						format!("deserialization error: {e}"),
					))
		})?)
			}
		}
		impl $crate::script::AipToLua for $ty {
	fn to_lua(self, lua: &mlua::Lua) -> $crate::Result<mlua::Value> {
				let serde_value =
					serde_json::to_value(self).map_err(|e| $crate::script::HandlerError::new(e.to_string()))?;
		Ok(<mlua::Value as $crate::script::LuaJsonExt>::x_from_json_value(lua, serde_value)
			.map_err(|e| $crate::script::HandlerError::new(e.to_string()))?)
			}
		}
	};
}

// endregion: --- Convenience macro

// region:    --- ToLua implementations

impl AipToLua for Value {
	fn to_lua(self, _lua: &Lua) -> Result<Value> {
		Ok(self)
	}
}

impl AipToLua for String {
	fn to_lua(self, lua: &Lua) -> Result<Value> {
		Ok(Value::String(
			lua.create_string(&self).map_err(|e| HandlerError::new(e.to_string()))?,
		))
	}
}

impl AipToLua for i64 {
	fn to_lua(self, _lua: &Lua) -> Result<Value> {
		Ok(Value::Integer(self))
	}
}

impl AipToLua for f64 {
	fn to_lua(self, _lua: &Lua) -> Result<Value> {
		Ok(Value::Number(self))
	}
}

impl AipToLua for bool {
	fn to_lua(self, _lua: &Lua) -> Result<Value> {
		Ok(Value::Boolean(self))
	}
}

impl<T: AipToLua> AipToLua for Option<T> {
	fn to_lua(self, lua: &Lua) -> Result<Value> {
		match self {
			None => Ok(Value::NULL),
			Some(v) => v.to_lua(lua),
		}
	}
}

impl<T: AipToLua> AipToLua for Vec<T> {
	fn to_lua(self, lua: &Lua) -> Result<Value> {
		let table = lua.create_table().map_err(|e| HandlerError::new(e.to_string()))?;
		for (i, v) in self.into_iter().enumerate() {
			table.set(i + 1, v.to_lua(lua)?).map_err(|e| HandlerError::new(e.to_string()))?;
		}
		Ok(Value::Table(table))
	}
}

impl<T: AipToLua> AipToLua for HashMap<String, T> {
	fn to_lua(self, lua: &Lua) -> Result<Value> {
		let table = lua.create_table().map_err(|e| HandlerError::new(e.to_string()))?;
		for (k, v) in self {
			table.set(k, v.to_lua(lua)?).map_err(|e| HandlerError::new(e.to_string()))?;
		}
		Ok(Value::Table(table))
	}
}

impl AipToLua for serde_json::Value {
	fn to_lua(self, lua: &Lua) -> Result<Value> {
		Ok(<mlua::Value as crate::script::LuaJsonExt>::x_from_json_value(lua, self)
			.map_err(|e| HandlerError::new(e.to_string()))?)
	}
}

// endregion: --- ToLua implementations

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::script::HandlerError;
	use mlua::Lua;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	fn lua() -> Lua {
		Lua::new()
	}

	// region:    --- FromLua / ToLua round‑trip primitives

	#[test]
	fn test_roundtrip_string() -> TestResult<()> {
		let l = lua();
		let original = "hello".to_string();
		let lua_val = original.clone().to_lua(&l)?;
		let back = String::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	#[test]
	fn test_roundtrip_i64() -> TestResult<()> {
		let l = lua();
		let original: i64 = 42;
		let lua_val = original.to_lua(&l)?;
		let back = i64::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	#[test]
	fn test_roundtrip_f64() -> TestResult<()> {
		let l = lua();
		let original: f64 = 3.18;
		let lua_val = original.to_lua(&l)?;
		let back = f64::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	#[test]
	fn test_roundtrip_bool() -> TestResult<()> {
		let l = lua();
		let original = true;
		let lua_val = original.to_lua(&l)?;
		let back = bool::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	// endregion: --- primitives

	// region:    --- Option<T>

	#[test]
	fn test_roundtrip_option_some_i64() -> TestResult<()> {
		let l = lua();
		let original: Option<i64> = Some(7);
		let lua_val = original.to_lua(&l)?;
		let back = Option::<i64>::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	#[test]
	fn test_roundtrip_option_none() -> TestResult<()> {
		let l = lua();
		let original: Option<i64> = None;
		let lua_val = original.to_lua(&l)?;
		// None → Value::NULL, which from_lua should turn back into None
		let back = Option::<i64>::from_lua(&l, lua_val)?;
		assert_eq!(back, None);
		Ok(())
	}

	#[test]
	fn test_from_lua_nil_sentinel_into_none() -> TestResult<()> {
		let l = lua();
		let nil_val: mlua::Value = Value::Nil;
		// Value::Nil is treated as None by Option::from_lua.
		let back = Option::<String>::from_lua(&l, nil_val)?;
		assert_eq!(back, None);
		Ok(())
	}

	// endregion: --- Option

	// region:    --- Vec<T>

	#[test]
	fn test_roundtrip_vec_i64() -> TestResult<()> {
		let l = lua();
		let original: Vec<i64> = vec![1, 2, 3];
		let lua_val = original.clone().to_lua(&l)?;
		let back = Vec::<i64>::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	// endregion: --- Vec

	// region:    --- HashMap<String, T>

	#[test]
	fn test_roundtrip_hashmap_string_i64() -> TestResult<()> {
		let l = lua();
		let mut original = HashMap::<String, i64>::new();
		original.insert("a".into(), 10);
		original.insert("b".into(), 20);
		let lua_val = original.clone().to_lua(&l)?;
		let back = HashMap::<String, i64>::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	// endregion: --- HashMap

	// region:    --- serde_json::Value

	#[test]
	fn test_roundtrip_json_value() -> TestResult<()> {
		let l = lua();
		let original: serde_json::Value = serde_json::json!({"key": [1, "two", null]});
		let lua_val = original.clone().to_lua(&l)?;
		let back = serde_json::Value::from_lua(&l, lua_val)?;
		assert_eq!(back, original);
		Ok(())
	}

	// endregion: --- serde_json::Value

	// region:    --- Error cases

	#[test]
	fn test_from_lua_wrong_type_string() {
		let l = lua();
		let number_val = mlua::Value::Integer(1);
		let err = String::from_lua(&l, number_val).unwrap_err();
		let msg = match err {
			crate::Error::Custom(msg) => msg,
			other => panic!("expected Custom error, got {:?}", other),
		};
		assert_eq!(msg, "expected string");
	}

	#[test]
	fn test_from_lua_wrong_type_i64() {
		let l = lua();
		let str_val = mlua::Value::String(l.create_string("nope").unwrap());
		let err = i64::from_lua(&l, str_val).unwrap_err();
		let handler_err = match err {
			crate::Error::Handler(h) => h,
			other => panic!("expected Handler error, got {:?}", other),
		};
		let inner = handler_err.get::<String>().expect("should contain string error");
		assert_eq!(inner.as_str(), "expected integer");
	}

	// endregion: --- Error cases
}

// endregion: --- Tests
