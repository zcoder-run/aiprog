use crate::Result;
use mlua::{Lua, LuaSerdeExt};

pub struct LuaEngine {
	lua: Lua,
}

impl LuaEngine {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn exec(&self, code: &str) -> Result<serde_json::Value> {
		let value = self.lua.load(code).eval()?;
		let value = self.lua.from_value(value)?;

		Ok(value)
	}
}

impl Default for LuaEngine {
	fn default() -> Self {
		Self { lua: Lua::new() }
	}
}
