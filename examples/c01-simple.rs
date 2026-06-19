use aiprog::ScriptEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let script_engine = ScriptEngine::new()?;

	let result = script_engine.exec(
		r#"
		local extra_text = '{"param_1": "value-1"}'
		local extra = aip.json.parse({text = extra_text})

		return {
			message = "Hello from Lua",
			count = 3,
			ok = true,
			extra = extra,
		}
		"#,
	)?;

	println!("{result:#?}");

	Ok(())
}
