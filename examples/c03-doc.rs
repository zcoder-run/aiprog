use std::fs;

use aiprog::{AipRegistry, ScriptEngine};
use simple_fs::{SPath, ensure_file_dir};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let engine_template = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let doc = engine_template.generate_doc()?;

	println!("{doc}");

	let out_file = SPath::new("examples/.out/c03-doc.md");
	ensure_file_dir(&out_file)?;
	fs::write(out_file, doc)?;

	Ok(())
}
