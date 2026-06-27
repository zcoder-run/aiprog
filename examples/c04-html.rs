use aiprog::ScriptEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let script_engine = ScriptEngine::new()?;

	println!("== aip.html.slim\n");
	let res = script_engine.exec(
		r#"
		local html_string = [[
		<html><head></head><body><p>Hello &amp; welcome!</p>
    <a class="blink" style="background: red" href="https://example.com">good&nbsp;link</a><script>some_stuff()</script></body></html>
		]]

		local html_slimmed = aip.html.slim({html = html_string, indent = 12})
		return html_slimmed
		"#,
	)?;

	let res = res.as_str().ok_or("No string response")?;
	println!("{res}");

	Ok(())
}
