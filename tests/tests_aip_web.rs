mod _support;

use aiprog::{AipRegistry, EngineTemplate, RunningContext};
use serde_json::json;

use _support::{TestResult, TestServerBuilder};

#[tokio::test]
async fn test_aip_web_get_parse_json_simple() -> TestResult {
	// -- Setup & Fixtures
	let server = TestServerBuilder::default()
		.with_header("Content-Type", "application/json")
		.with_body(r#"{"hello":"world"}"#.as_bytes())
		.start()
		.await?;
	let url = server.path_url("/json");
	let engine = EngineTemplate::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(
			&format!(r#"return aip.web.get{{ url = "{url}", parse = true }}"#),
			RunningContext::default(),
		)
		.await?
		.result?;

	// -- Check
	assert_eq!(result["data"], json!({ "hello": "world" }));
	assert_eq!(result["status"], 200);
	assert_eq!(result["success"], true);
	assert_eq!(
		result["url"].as_str().ok_or("Expected response URL")?,
		url
	);
	assert_eq!(server.request()?.method, "GET");
	assert_eq!(server.request()?.path, "/json");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_aip_web_post_json_request_body() -> TestResult {
	// -- Setup & Fixtures
	let server = TestServerBuilder::default()
		.with_header("Content-Type", "application/json")
		.with_body(r#"{"result":"created"}"#.as_bytes())
		.start()
		.await?;
	let url = server.path_url("/records");
	let engine = EngineTemplate::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(
			&format!(
				r#"return aip.web.post{{ url = "{url}", json = {{ name = "Ada", active = true }}, parse = true }}"#
			),
			RunningContext::default(),
		)
		.await?
		.result?;
	let request = server.request()?;

	// -- Check
	assert_eq!(result["data"], json!({ "result": "created" }));
	assert_eq!(result["status"], 200);
	assert_eq!(request.method, "POST");
	assert_eq!(request.path, "/records");
	assert!(
		request
			.headers
			.get("content-type")
			.is_some_and(|content_type| content_type.starts_with("application/json"))
	);
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&request.body)?,
		json!({ "name": "Ada", "active": true })
	);

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_aip_web_get_http_error_response() -> TestResult {
	// -- Setup & Fixtures
	let server = TestServerBuilder::default()
		.with_status(404)
		.with_header("Content-Type", "text/plain")
		.with_body("missing".as_bytes())
		.start()
		.await?;
	let url = server.path_url("/missing");
	let engine = EngineTemplate::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(
			&format!(r#"return aip.web.get{{ url = "{url}" }}"#),
			RunningContext::default(),
		)
		.await?
		.result?;

	// -- Check
	assert_eq!(result["data"], "missing");
	assert_eq!(result["status"], 404);
	assert_eq!(result["success"], false);
	assert!(
		result["error"]
			.as_str()
			.is_some_and(|error| error.contains("status 404"))
	);

	server.close().await?;

	Ok(())
}
