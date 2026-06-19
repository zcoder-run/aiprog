use aiprog::registry::AipRegistry;
use aiprog::script::AipApiError;
use aiprog::{AipFromLua, AipIntoLua, AipParams, AipResponse, ScriptEngine};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use value_ext::JsonValueExt as _;

// region:    --- Types

#[derive(Debug, Deserialize, Serialize, JsonSchema, AipFromLua, AipParams)]
struct GreetingParams {
	name: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, AipIntoLua, AipResponse)]
struct GreetingResult(String);

fn custom_greetings(params: GreetingParams) -> core::result::Result<GreetingResult, AipApiError> {
	Ok(GreetingResult(format!("Hello {}", params.name)))
}

// endregion: --- Handler

// region:    --- Main

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut registry = AipRegistry::default();
	registry.register_sync("custom.greeting", custom_greetings)?;

	let script_engine = ScriptEngine::from_registry(registry)?;

	let result = script_engine.exec(
		r#"
        local res = custom.greeting({name = "World"})
        return res
        "#,
	)?;

	println!("{}", result.x_pretty()?);

	Ok(())
}

// endregion: --- Main
