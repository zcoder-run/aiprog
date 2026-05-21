use crate::support::W;
use mlua::{IntoLua, Lua, Value};

impl IntoLua for W<&String> {
	fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
		Ok(Value::String(lua.create_string(self.0)?))
	}
}

impl IntoLua for W<String> {
	fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
		Ok(Value::String(lua.create_string(&self.0)?))
	}
}

impl IntoLua for W<Vec<String>> {
	fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
		let table = lua.create_table()?;
		for (i, s) in self.0.into_iter().enumerate() {
			table.set(i + 1, s)?;
		}
		Ok(Value::Table(table))
	}
}

impl IntoLua for W<Vec<Vec<String>>> {
	fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
		let top_vec = self.0;
		let outer = lua.create_table()?;
		for (i, row) in top_vec.into_iter().enumerate() {
			let inner = lua.create_table()?;
			for (j, cell) in row.into_iter().enumerate() {
				inner.set(j + 1, cell)?;
			}
			outer.set(i + 1, inner)?;
		}
		Ok(Value::Table(outer))
	}
}
