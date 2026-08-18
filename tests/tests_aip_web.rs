mod _support;

use aiprog::{AipRegistry, RunningContext, ScriptEngine};
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
	let engine = ScriptEngine::builder()
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
	assert_eq!(result["url"].as_str().ok_or("Expected response URL")?, url);
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
	let engine = ScriptEngine::builder()
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
	let engine = ScriptEngine::builder()
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
	assert!(result["error"].as_str().is_some_and(|error| error.contains("status 404")));

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_aip_web_get_query_params() -> TestResult {
	// -- Setup & Fixtures
	let server = TestServerBuilder::default().with_body("ok".as_bytes()).start().await?;
	let url = server.path_url("/search");
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(
			&format!(
				r#"return aip.web.get{{ url = "{url}", query_params = {{ ["space key"] = "hello world", tag = {{ "rust", "web" }} }} }}"#
			),
			RunningContext::default(),
		)
		.await?
		.result?;
	let request = server.request()?;
	let query = request.path.split_once('?').ok_or("Expected query string")?.1;
	let query_pairs = url::form_urlencoded::parse(query.as_bytes())
		.map(|(name, value)| (name.into_owned(), value.into_owned()))
		.collect::<Vec<_>>();

	// -- Check
	assert_eq!(result["data"], "ok");
	assert_eq!(result["success"], true);
	assert_eq!(request.method, "GET");
	assert!(request.path.starts_with("/search?"));
	assert!(request.path.contains("space+key=hello+world"));
	assert!(query_pairs.contains(&("space key".to_string(), "hello world".to_string())));
	assert!(query_pairs.contains(&("tag".to_string(), "rust".to_string())));
	assert!(query_pairs.contains(&("tag".to_string(), "web".to_string())));

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_aip_web_post_query_params() -> TestResult {
	// -- Setup & Fixtures
	let server = TestServerBuilder::default().with_body("created".as_bytes()).start().await?;
	let url = server.path_url("/records?existing=keep");
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	// -- Exec
	let result = engine
		.exec(
			&format!(
				r#"return aip.web.post{{ url = "{url}", body = "payload", query_params = {{ ["filter/value"] = "a&b=c", id = "42", tag = {{ "one", "two" }} }} }}"#
			),
			RunningContext::default(),
		)
		.await?
		.result?;
	let request = server.request()?;
	let query = request.path.split_once('?').ok_or("Expected query string")?.1;
	let query_pairs = url::form_urlencoded::parse(query.as_bytes())
		.map(|(name, value)| (name.into_owned(), value.into_owned()))
		.collect::<Vec<_>>();

	// -- Check
	assert_eq!(result["data"], "created");
	assert_eq!(result["success"], true);
	assert_eq!(request.method, "POST");
	assert!(request.path.starts_with("/records?existing=keep&"));
	assert!(request.path.contains("filter%2Fvalue=a%26b%3Dc"));
	assert!(query_pairs.contains(&("existing".to_string(), "keep".to_string())));
	assert!(query_pairs.contains(&("filter/value".to_string(), "a&b=c".to_string())));
	assert!(query_pairs.contains(&("id".to_string(), "42".to_string())));
	assert!(query_pairs.contains(&("tag".to_string(), "one".to_string())));
	assert!(query_pairs.contains(&("tag".to_string(), "two".to_string())));

	server.close().await?;

	Ok(())
}
