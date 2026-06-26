use lazy_regex::regex;
use serde::Serialize;
use std::borrow::Cow;
use std::fmt;

pub type HandlerResult<T, K = KindNone> = core::result::Result<T, HandlerError<K>>;

// region:    --- KindNone

/// Marker type for HandlerError kind that serializes to nothing.
///
/// When used as the kind in `HandlerError`, the `kind` field is omitted
/// from serialization, producing `{"message": "..."}`.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema, Default)]
pub struct KindNone;

impl fmt::Display for KindNone {
	fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
		Ok(())
	}
}

// endregion: --- KindNone

// region:    --- HandlerError

/// Generic handler error.
///
/// When `K` is `KindNone` (the default), the serialized output is
/// `{"message": "..."}`. When `K` is a custom kind implementing `Display`,
/// the output is `{"kind": "...", "message": "..."}`.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct HandlerError<K: std::fmt::Display + 'static = KindNone> {
	#[serde(
		serialize_with = "serialize_kind",
		skip_serializing_if = "kind_is_none"
	)]
	kind: K,
	message: String,
}

// endregion: --- HandlerError

// region:    --- Display

impl<K: fmt::Display + 'static> fmt::Display for HandlerError<K> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if kind_is_none(&self.kind) {
			f.write_str(&self.message)
		} else {
			write!(f, "{}: {}", self.kind, self.message)
		}
	}
}

// endregion: --- Display

// region:    --- Serialization helpers

fn kind_is_none<K: 'static>(_: &K) -> bool {
	std::any::TypeId::of::<K>() == std::any::TypeId::of::<KindNone>()
}

fn serialize_kind<K: fmt::Display, S: serde::Serializer>(kind: &K, serializer: S) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&kind.to_string())
}

// endregion: --- Serialization helpers

// region:    --- Construction

impl HandlerError<KindNone> {
	/// Create a new `HandlerError` with no kind (KindNone) and the given message.
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			kind: KindNone,
			message: message.into(),
		}
	}
}

impl<K: std::fmt::Display> HandlerError<K> {
	/// Create a new `HandlerError` with a specific kind and message.
	pub fn with_kind(kind: K, message: impl Into<String>) -> Self {
		Self {
			kind,
			message: message.into(),
		}
	}
}

// endregion: --- Construction

// region:    --- Convenience constructors for KindNone

impl HandlerError<KindNone> {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::new(val)
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::new(err.to_string())
	}

	pub fn cc(context: impl Into<String>, cause: impl std::fmt::Display) -> Self {
		Self::new(format!("{}: {}", context.into(), cause))
	}
}

// endregion: --- Convenience constructors

// region:    --- Lua conversions

impl HandlerError<KindNone> {
	/// Convert a normalized `HandlerError` into an `mlua::Error`.
	pub fn into_lua_error(self) -> mlua::Error {
		mlua::Error::RuntimeError(self.message)
	}

	/// Build a `HandlerError` from a Lua error, enriching stack traces with the provided script source.
	pub fn from_lua_error_with_script(lua_error: &mlua::Error, script: &str) -> Self {
		let mut buff: Vec<String> = Vec::new();
		for item in lua_error.chain() {
			buff.push(process_stack_with_script(&item.to_string(), script));
		}
		HandlerError::new(buff.join("\n"))
	}
}

// endregion: --- Lua conversions

// region:    --- Private helpers

fn process_stack_with_script(stack: &str, script: &str) -> String {
	let script_lines: Vec<&str> = script.lines().collect();
	let mut buff: Vec<Cow<str>> = Vec::new();
	let rx = regex!(r#"src/script/lua_engine\s*\.[^\n]*:(\d+):"#);
	for line in stack.lines() {
		if rx.is_match(line) {
			let replaced_line = rx.replace_all(line, |caps: &regex::Captures| {
				if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
					if let Some(script_line) = script_lines.get(num - 1) {
						let script_line = script_line.trim();
						Cow::from(format!("At line {num} '{script_line}'"))
					} else {
						Cow::from(format!("Line({num})"))
					}
				} else {
					Cow::from("")
				}
			});
			buff.push(replaced_line);
		} else {
			buff.push(line.into());
		}
	}
	buff.join("\n")
}

// endregion: --- Private helpers

// region:    --- From conversions

impl From<String> for HandlerError<KindNone> {
	fn from(s: String) -> Self {
		HandlerError::new(s)
	}
}

impl From<&str> for HandlerError<KindNone> {
	fn from(s: &str) -> Self {
		HandlerError::new(s.to_string())
	}
}

impl From<&String> for HandlerError<KindNone> {
	fn from(s: &String) -> Self {
		HandlerError::new(s.clone())
	}
}

impl From<crate::Error> for HandlerError {
	fn from(e: crate::Error) -> Self {
		HandlerError::new(e.to_string())
	}
}

impl From<serde_json::Value> for HandlerError {
	fn from(v: serde_json::Value) -> Self {
		HandlerError::new(v.to_string())
	}
}

impl From<mlua::Error> for HandlerError {
	fn from(e: mlua::Error) -> Self {
		HandlerError::new(e.to_string())
	}
}

// endregion: --- From conversions

// region:    --- Error Boilerplate

impl<K: fmt::Debug + fmt::Display> std::error::Error for HandlerError<K> {}

// endregion: --- Error Boilerplate
