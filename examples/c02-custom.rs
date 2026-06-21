use aiprog::{AipFromLua, AipIntoLua, AipOutput, AipParams, AipRegistry, ApiError, Error, ScriptEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use value_ext::JsonValueExt as _;

// region:    --- Types

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GreetingParams {
	/// Name of to be greeted
	name: String,
}

impl AipFromLua for GreetingParams {
	fn from_lua(lua: &mlua::Lua, value: mlua::Value) -> aiprog::Result<Self> {
		let table = match value {
			mlua::Value::Table(t) => t,
			_ => return Err(Error::custom("expected a table")),
		};
		let name_value = table.get::<mlua::Value>("name").map_err(|e| Error::custom(format!("{}", e)))?;
		let name: String = AipFromLua::from_lua(lua, name_value)?;
		Ok(GreetingParams { name })
	}
}

impl AipParams for GreetingParams {}

/// The greeting
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GreetingResult(String);

impl AipIntoLua for GreetingResult {
	fn into_lua(self, lua: &mlua::Lua) -> aiprog::Result<mlua::Value> {
		let s = lua.create_string(&self.0).map_err(|e| Error::custom(format!("{}", e)))?;
		Ok(mlua::Value::String(s))
	}
}

impl AipOutput for GreetingResult {}

fn custom_greetings(params: GreetingParams) -> core::result::Result<GreetingResult, ApiError> {
	Ok(GreetingResult(format!("Hello {}", params.name)))
}

// endregion: --- Handler

// region:    --- Main

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut registry = AipRegistry::from_aip_modules()?;
	registry.register_sync("custom.greeting", custom_greetings)?;

	let script_engine = ScriptEngine::from_registry(registry)?;

	let result = script_engine.exec(
		r#"
        local res = custom.greeting({name = "World"})
        return res
        "#,
	)?;

	println!("{}", result.x_pretty()?);

	// -- print doc

	let doc = script_engine.generate_doc()?;

	println!("\n=== Doc:\n{doc}");

	Ok(())
}

// endregion: --- Main
