//! Lua Management implementaitons for the crate::Error

// Deprecated: backward-compatibility shim. use crate::ScriptError methods.

use crate::Error;
use crate::registry::HandlerError;
use std::sync::Arc;

impl Error {
	pub fn from_error_with_script(lua_error: &mlua::Error, script: &str) -> Error {
		let handler_err = HandlerError::from_lua_error_with_script(lua_error, script);
		Error::Handler(handler_err)
	}
}

// region:    --- Froms

/// Do the From mlua error without script
impl From<&mlua::Error> for Error {
	fn from(lua_error: &mlua::Error) -> Self {
		let mut buff: Vec<String> = Vec::new();
		for item in lua_error.chain() {
			if let Some(lua_item) = item.downcast_ref::<mlua::Error>() {
				buff.push(format!("Lua error chain item\n - {lua_item}"))
			} else {
				buff.push(format!("Other error chain item\n - {item}"))
			}
		}
		let msg = buff.join("\n");
		// Note: here is Self::lua, it gets a stackoverflow
		Self::custom(msg)
	}
}

impl From<Error> for mlua::Error {
	fn from(value: Error) -> Self {
		// TODO - revisit
		#[allow(clippy::arc_with_non_send_sync)]
		mlua::Error::ExternalError(Arc::new(value))
	}
}

// endregion: --- Froms
