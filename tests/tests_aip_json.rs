// Integration tests for Aiprog JSON handling via the AIProg Lua engine.

use aiprog::{AipRegistry, EngineTemplate, RunningContext};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_aiprog_run_json_simple_ok() -> TestResult {
	// -- Setup & Fixtures
	let json_text = std::fs::read_to_string("tests/data/json/01-simple.json")?;
	let json_text = serde_json::to_string(&json_text)?;
	let lua_code = format!(
		r#"
        local parsed = aip.json.parse({{text = {json_text}}})
        return parsed
    "#
	);

	let engine = EngineTemplate::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(&lua_code, RunningContext::default())
		.await?
		.result?;

	// -- Check
	let expected = json!({
		"simple": "json",
		"num": 123,
		"extra": { "values": ["one", "two"] }
	});
	assert_eq!(result, expected);

	Ok(())
}
