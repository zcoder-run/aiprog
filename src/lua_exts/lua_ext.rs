use crate::{Error, Result};
use mlua::{BorrowedStr, Table, Value};

/// Convenient Lua Value extension
///
/// TODO: Will need to handle the case where the found value is not of correct type. Probably should return `Result<Option<>>`
#[allow(dead_code)]
pub trait LuaExt {
	/// return true if NULL, Nil, or None (for Option<Value>)
	fn x_is_null(&self) -> bool;

	fn x_as_lua_str(&self) -> Option<BorrowedStr>;

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

	/// Return the Lua value for a key, failing loudly on invalid access.
	/// - Absent key or `nil` value returns `Ok(None)`
	/// - Non-table `self` returns `Err`
	fn x_try_get_value(&self, key: &str) -> Result<Option<Value>>;

	/// Result variants of the `x_get_*` accessors.
	/// - Absent key or `nil` value returns `Ok(None)`
	/// - Present but wrong-typed value returns `Err` with field name, expected type,
	///   actual Lua type, and a truncated value preview (about 80 chars)
	fn x_try_get_string(&self, key: &str) -> Result<Option<String>>;
	fn x_try_get_bool(&self, key: &str) -> Result<Option<bool>>;
	fn x_try_get_i64(&self, key: &str) -> Result<Option<i64>>;
	fn x_try_get_f64(&self, key: &str) -> Result<Option<f64>>;

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

	fn x_as_lua_str(&self) -> Option<BorrowedStr> {
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

	fn x_try_get_value(&self, key: &str) -> Result<Option<Value>> {
		let table = self.as_table().ok_or_else(|| not_a_table_err(key, self))?;
		table.x_try_get_value(key)
	}

	fn x_try_get_string(&self, key: &str) -> Result<Option<String>> {
		let table = self.as_table().ok_or_else(|| not_a_table_err(key, self))?;
		table.x_try_get_string(key)
	}

	fn x_try_get_bool(&self, key: &str) -> Result<Option<bool>> {
		let table = self.as_table().ok_or_else(|| not_a_table_err(key, self))?;
		table.x_try_get_bool(key)
	}

	fn x_try_get_i64(&self, key: &str) -> Result<Option<i64>> {
		let table = self.as_table().ok_or_else(|| not_a_table_err(key, self))?;
		table.x_try_get_i64(key)
	}

	fn x_try_get_f64(&self, key: &str) -> Result<Option<f64>> {
		let table = self.as_table().ok_or_else(|| not_a_table_err(key, self))?;
		table.x_try_get_f64(key)
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

	fn x_as_lua_str(&self) -> Option<BorrowedStr> {
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

	fn x_try_get_value(&self, key: &str) -> Result<Option<Value>> {
		let val = self
			.get::<Value>(key)
			.map_err(|err| Error::cc(format!("Fail to get property '{key}'"), err))?;
		if val.is_nil() { Ok(None) } else { Ok(Some(val)) }
	}

	fn x_try_get_string(&self, key: &str) -> Result<Option<String>> {
		try_get_typed(self, key, "string", |v| v.x_to_string())
	}

	fn x_try_get_bool(&self, key: &str) -> Result<Option<bool>> {
		try_get_typed(self, key, "boolean", |v| v.x_as_bool())
	}

	fn x_try_get_i64(&self, key: &str) -> Result<Option<i64>> {
		try_get_typed(self, key, "integer", |v| v.x_as_i64())
	}

	fn x_try_get_f64(&self, key: &str) -> Result<Option<f64>> {
		try_get_typed(self, key, "number", |v| v.x_as_f64())
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
// region:    --- Try Get Support

/// Max chars for the value preview included in type mismatch errors.
const PREVIEW_MAX_CHARS: usize = 80;

/// Get `key` from `table` and extract it as the expected type.
/// - Absent key or `nil` value returns `Ok(None)`
/// - Present but non-extractable value returns a type mismatch `Err`
fn try_get_typed<T>(
	table: &Table,
	key: &str,
	expected: &str,
	extract: impl Fn(&Value) -> Option<T>,
) -> Result<Option<T>> {
	let val = table
		.get::<Value>(key)
		.map_err(|err| Error::cc(format!("Fail to get property '{key}'"), err))?;
	if val.is_nil() {
		return Ok(None);
	}
	match extract(&val) {
		Some(v) => Ok(Some(v)),
		None => Err(type_mismatch_err(key, expected, &val)),
	}
}

/// Build the error for a present-but-wrong-typed property.
fn type_mismatch_err(key: &str, expected: &str, val: &Value) -> Error {
	let actual = val.type_name();
	let preview = value_preview(val);
	Error::custom(format!(
		"Property '{key}' expected to be of type '{expected}', but was of type '{actual}' (value: {preview})"
	))
}

/// Build the error when trying to get a property on a non-table value.
fn not_a_table_err(key: &str, val: &Value) -> Error {
	Error::custom(format!(
		"Cannot get property '{key}' because the value is not a table but of type '{}'",
		val.type_name()
	))
}

/// Render a short, truncated preview of a Lua value for error messages.
fn value_preview(val: &Value) -> String {
	let raw = match val {
		Value::String(s) => format!("\"{}\"", s.to_string_lossy()),
		Value::Integer(num) => num.to_string(),
		Value::Number(num) => num.to_string(),
		Value::Boolean(b) => b.to_string(),
		other => format!("{other:?}"),
	};
	if raw.chars().count() > PREVIEW_MAX_CHARS {
		let truncated: String = raw.chars().take(PREVIEW_MAX_CHARS).collect();
		format!("{truncated}...")
	} else {
		raw
	}
}

// endregion: --- Try Get Support
