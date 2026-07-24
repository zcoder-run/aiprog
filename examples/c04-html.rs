use aiprog::{AipRegistry, ScriptEngine, RunningContext};
use value_ext::JsonValueExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let engine_template = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	println!("== aip.html.slim\n");
	let res = engine_template.exec(
		r#"
		local html_string = [[
		<html><head></head><body><p>Hello &amp; welcome!</p>
    <a class="blink" style="background: red" href="https://example.com">good&nbsp;link</a><script>some_stuff()</script></body></html>
		]]

		local slimmed = aip.html.slim({html = html_string, indent = 12})

		local md = aip.html.to_md({html = slimmed})

		return {
			slimmed = slimmed,
			md = md
		}
		"#,
		RunningContext::default(),
	).await?.result?;

	let slimmed = res.x_get_str("slimmed")?;
	let md = res.x_get_str("md")?;

	println!("=== slimmed:\n{slimmed}");

	println!("\n\n=== md:\n{md}");

	Ok(())
}
