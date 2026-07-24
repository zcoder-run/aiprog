use aiprog::{AipRegistry, RunningContext, ScriptEngine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let engine_template = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	println!("== aip.json.parse\n");
	let result = engine_template
		.exec(
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
			RunningContext::default(),
		)
		.await?
		.result?;

	println!("{result:#?}");

	println!("== aip.json.stringify (pretty = true)\n");
	let result = engine_template
		.exec(
			r#"
		local data = {
			message = "Hello from lua",
			count = 3,
			ok = true,
			nest = {
				one = "first"
			}
		}

		local text = aip.json.stringify({data = data, pretty = true})
		return text
		"#,
			RunningContext::default(),
		)
		.await?
		.result?;
	let result = result.as_str().ok_or("no result string")?;

	println!("{result}");

	Ok(())
}
