use std::fs;

use aiprog::ScriptEngine;
use simple_fs::{SPath, ensure_file_dir};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let script_engine = ScriptEngine::new()?;

	let doc = script_engine.generate_doc()?;

	println!("{doc}");

	let out_file = SPath::new("examples/.out/c03-doc.md");
	ensure_file_dir(&out_file)?;
	fs::write(out_file, doc)?;

	Ok(())
}
