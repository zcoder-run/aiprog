use mlua::{BorrowedStr, Table, Value};

/// Convenient Lua Value extension
///
/// TODO: Will need to handle the case where the found value is not of correct type. Probably should return `Result<Option<>>`
#[allow(dead_code)]
pub trait LuaExt {
	/// return true if NULL, Nil, or None (for Option<Value>)
	fn x_is_null(&self) -> bool;

	fn x_as_lua_str(&self) -> Option<BorrowedStr<'_>>;

	/// Note: Will round if floating number
	fn x_as_i64(&self) -> Option<i64>;
	fn x_as_f64(&self) -> Option<f64>;
	fn x_as_bool(&self) -> Option<bool>;

	fn x_to_string(&self) -> Option<String>;

	/// Return the Lua value for a key.
	/// NOTE: Will return None if value is Nil
	fn x_get_value(&self, key: &str) -> Option<Value>;
	fn x_get_string(&self, key: &str) -> Option<String>;
	fn x_get_bool(&self, key: &str) -> Option<bool>;
	fn x_get_i64(&self, key: &str) -> Option<i64>;
	fn x_get_f64(&self, key: &str) -> Option<f64>;

	/// Returns the sequential list part of a table as an owned Vec<Value>.
	///
	/// - If `self` is not a table, returns `None`.
	/// - If it is a table and has key `1`, returns the contiguous sequence from 1 until the first `nil`.
	/// - If it is a table but does not have key `1`, returns `Some(vec![])` (empty list).
	///
	/// The returned values are owned (cloned), so they can outlive the original Lua value.
	fn x_as_list(&self) -> Option<Vec<Value>>;
}

impl LuaExt for Value {
	fn x_is_null(&self) -> bool {
		self == &Value::NULL || self == &Value::Nil
	}

	fn x_as_lua_str(&self) -> Option<BorrowedStr<'_>> {
		self.as_string().and_then(|s| s.to_str().ok())
	}

	fn x_as_i64(&self) -> Option<i64> {
		match self {
			Value::Integer(num) => Some(*num),
			Value::Number(num) => Some(num.round() as i64),
			_ => None,
		}
	}
	fn x_as_f64(&self) -> Option<f64> {
		match self {
			Value::Integer(num) => Some(*num as f64),
			Value::Number(num) => Some(*num),
			_ => None,
		}
	}
	fn x_as_bool(&self) -> Option<bool> {
		self.as_boolean()
	}

	fn x_to_string(&self) -> Option<String> {
		self.as_string().map(|v| v.to_string_lossy())
	}

	fn x_get_value(&self, key: &str) -> Option<Value> {
		let table = self.as_table()?;
		let val = table.get::<Value>(key).ok()?;
		if val.is_nil() { None } else { Some(val) }
	}

	fn x_get_string(&self, key: &str) -> Option<String> {
		let table = self.as_table()?;
		let val = table.get::<Value>(key).ok()?;
		let val = val.x_as_lua_str()?;
		Some(val.to_string())
	}

	fn x_get_bool(&self, key: &str) -> Option<bool> {
		let table = self.as_table()?;
		let val = table.get::<Value>(key).ok()?;
		let val = val.as_boolean()?;
		Some(val)
	}

	fn x_get_i64(&self, key: &str) -> Option<i64> {
		let table = self.as_table()?;
		let val = table.get::<Value>(key).ok()?;
		let val = val.as_i64()?;
		Some(val)
	}

	fn x_get_f64(&self, key: &str) -> Option<f64> {
		let table = self.as_table()?;
		let val = table.get::<Value>(key).ok()?;
		let val = val.as_f64()?;
		Some(val)
	}

	fn x_as_list(&self) -> Option<Vec<Value>> {
		let table = self.as_table()?;
		Some(table_as_list(table))
	}
}

impl LuaExt for Table {
	fn x_is_null(&self) -> bool {
		false
	}

	fn x_as_lua_str(&self) -> Option<BorrowedStr<'_>> {
		None
	}

	fn x_as_i64(&self) -> Option<i64> {
		None
	}
	fn x_as_f64(&self) -> Option<f64> {
		None
	}
	fn x_as_bool(&self) -> Option<bool> {
		None
	}

	fn x_to_string(&self) -> Option<String> {
		None
	}

	fn x_get_value(&self, key: &str) -> Option<Value> {
		let val = self.get::<Value>(key).ok()?;
		if val.is_nil() { None } else { Some(val) }
	}

	fn x_get_string(&self, key: &str) -> Option<String> {
		let val = self.get::<Value>(key).ok()?;
		let val = val.x_as_lua_str()?;
		Some(val.to_string())
	}

	fn x_get_bool(&self, key: &str) -> Option<bool> {
		let val = self.get::<Value>(key).ok()?;
		let val = val.as_boolean()?;
		Some(val)
	}

	fn x_get_i64(&self, key: &str) -> Option<i64> {
		let val = self.get::<Value>(key).ok()?;
		let val = val.as_i64()?;
		Some(val)
	}

	fn x_get_f64(&self, key: &str) -> Option<f64> {
		let val = self.get::<Value>(key).ok()?;
		let val = val.as_f64()?;
		Some(val)
	}

	fn x_as_list(&self) -> Option<Vec<Value>> {
		Some(table_as_list(self))
	}
}

/// Extract the sequence part of a Lua table (keys 1..N contiguous, stops at first nil).
#[allow(dead_code)]
fn table_as_list(table: &Table) -> Vec<Value> {
	table.sequence_values().filter_map(|v| v.ok()).collect()
}
