use aiprog::ScriptEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let script_engine = ScriptEngine::new()?;

	let doc = script_engine.generate_doc()?;

	println!("{doc}");

	Ok(())
}
