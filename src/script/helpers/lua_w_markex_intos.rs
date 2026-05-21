use crate::support::W;
use mlua::IntoLua;

impl IntoLua for W<markex::tag::TagElem> {
	/// Converts the `TagElem` instance into a Lua Value
	fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
		let this = self.0;

		let table = lua.create_table()?;
		table.set("tag", this.tag)?;
		table.set("attrs", this.attrs)?; // Note: Lua might need handling for Option<HashMap>
		table.set("content", this.content)?;
		Ok(mlua::Value::Table(table))
	}
}
