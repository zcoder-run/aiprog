use aiprog::ScriptEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let lua_engine = ScriptEngine::new();

	let result = lua_engine.exec(
		r#"
		return {
			message = "Hello from Lua",
			count = 3,
			ok = true
		}
		"#,
	)?;

	println!("{result:#?}");

	Ok(())
}
