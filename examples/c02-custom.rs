use aiprog::{
	AipFromLua, AipIntoLua, AipOutput, AipParams, AipRegistry, Error, HandlerResult, LuaExt, ScriptEngine, aip_handler,
	register_handler,
};
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
	fn from_lua(_lua: &mlua::Lua, value: mlua::Value) -> aiprog::Result<Self> {
		let name = value.x_get_string("name").ok_or("No .name value (required)")?;
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

/// Custom greeting handler.
#[aip_handler]
fn custom_greetings(params: GreetingParams) -> HandlerResult<GreetingResult> {
	Ok(GreetingResult(format!("Hello {}", params.name)))
}

// endregion: --- Handler

// region:    --- Main

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut registry = AipRegistry::from_aip_modules()?;
	register_handler!(registry, "custom.greeting", custom_greetings)?;

	let script_engine = ScriptEngine::from_registry(registry)?;

	let result = script_engine.exec(
		r#"
        local res = custom.greeting({name = "World"})
        return res
        "#,
	)?;

	println!("{}", result.x_pretty()?);

	// -- print doc

	// let doc = script_engine.generate_doc()?;

	// println!("\n=== Doc:\n{doc}");

	Ok(())
}

// endregion: --- Main
