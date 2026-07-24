use std::fs;

use aiprog::{
	AipFromLua, AipIntoLua, AipOutput, AipParams, AipRegistry, Error, HandlerCallContext, HandlerResult, LuaExt,
	RunningContext, ScriptEngine, aip_handler, register_handler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use simple_fs::{SPath, ensure_file_dir};
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
struct GreetingOutput(String);

impl AipIntoLua for GreetingOutput {
	fn into_lua(self, lua: &mlua::Lua) -> aiprog::Result<mlua::Value> {
		let s = lua.create_string(&self.0).map_err(|e| Error::custom(format!("{}", e)))?;
		Ok(mlua::Value::String(s))
	}
}

impl AipOutput for GreetingOutput {}

/// Custom greeting handler.
#[aip_handler]
fn custom_greetings(_call: HandlerCallContext, params: GreetingParams) -> HandlerResult<GreetingOutput> {
	Ok(GreetingOutput(format!("Hello {}", params.name)))
}

// endregion: --- Handler

// region:    --- Main

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut registry_builder = AipRegistry::from_aip_modules()?.to_builder();
	register_handler!(registry_builder, "custom.greeting", custom_greetings)?;
	let registry = registry_builder.build();

	let engine_template = ScriptEngine::builder().with_registry(registry).build()?;

	let result = engine_template
		.exec(
			r#"
        local res = custom.greeting({name = "World"})
        return res
        "#,
			RunningContext::default(),
		)
		.await?
		.result?;

	println!("{}", result.x_pretty()?);

	// -- Save doc

	let doc = engine_template.generate_doc()?;
	let out_file = SPath::new("examples/.out/c02-doc.md");
	ensure_file_dir(&out_file)?;
	fs::write(out_file, doc)?;

	Ok(())
}

// endregion: --- Main
