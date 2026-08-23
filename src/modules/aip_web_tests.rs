//! Tests for the aip.web module, following the same pattern as aip_json_tests.

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use value_ext::JsonValueExt as _;

use crate::AipRegistryBuilder;
use crate::_test_support;
use crate::modules::WebModule;

#[tokio::test]
async fn test_api_web_constants() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	// Install the constants (must be done after the functions are installed)
	crate::modules::aip_web::install_constants(&engine)?;

	// -- Exec
	let script = r#"
			local ua_aiprog = aip.web.UA_AIPROG
			local ua_browser = aip.web.UA_BROWSER
			return { ua_aiprog = ua_aiprog, ua_browser = ua_browser }
		"#;
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert_eq!(res["ua_aiprog"], "aiprog");
	assert_eq!(
		res["ua_browser"],
		"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36"
	);

	Ok(())
}

#[tokio::test]
async fn test_api_web_get_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "application/json")
		.body(r#"{"hello":"world"}"#)
		.start()
		.await?;
	let url = server.path_url("/test");

	// -- Exec
	let script = format!(r#"return aip.web.get{{ url = "{url}", parse = true }}"#,);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert_eq!(res.x_get_i64("status")?, 200);
	let hello = res.x_get_str("/data/hello").ok().ok_or("expected data.hello")?;
	assert_eq!(hello, "world");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_json() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "application/json")
		.body(r#"{"result":"success"}"#)
		.validate(|snap| {
			assert_eq!(snap.method, "POST", "Expected POST method");
			assert!(snap.body.contains(r#""key":"value""#), "Expected JSON body");
		})
		.start()
		.await?;
	let url = server.path_url("/test");

	// -- Exec
	let script = format!(r#"return aip.web.post{{ url = "{url}", body = {{ key = "value" }}, parse = true }}"#,);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert_eq!(res.x_get_i64("status")?, 200);
	assert_eq!(res.x_get_str("/data/result")?, "success");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_body() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "text/plain")
		.body("received")
		.validate(|snap| {
			assert_eq!(snap.method, "POST", "Expected POST method");
			assert!(snap.body.contains("hello world body"), "Expected raw body in request");
		})
		.start()
		.await?;
	let url = server.path_url("/test");

	// -- Exec
	let script = format!(r#"return aip.web.post{{ url = "{url}", body = "hello world body" }}"#,);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert_eq!(res.x_get_str("data")?, "received");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new().start().await?;
	let url = server.path_url("/test");
	server.close().await?; // shut down the server to force a connection error

	// -- Exec
	let script = format!(r#"return aip.web.post{{ url = "{url}", body = {{ key = "value" }} }}"#,);
	let result = _test_support::eval_script(&engine, &script).await;

	// -- Check
	assert!(result.is_err(), "Expected connection error but got Ok");

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_explicit_content_type_override_json() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "application/json")
		.body(r#"{"saved":true}"#)
		.validate(|snap| {
			assert_eq!(snap.method, "POST");
			assert_eq!(
				snap.headers.get("content-type").map(|s| s.as_str()),
				Some("application/vnd.custom+json")
			);
			assert_eq!(snap.body, r#"{"msg":"hello"}"#);
		})
		.start()
		.await?;
	let url = server.path_url("/custom-json");

	// -- Exec
	let script = format!(
		r#"return aip.web.post{{ url = "{url}", body = {{ msg = "hello" }}, content_type = "application/vnd.custom+json", parse = true }}"#,
	);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert!(res.x_get_bool("/data/saved")?);

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_explicit_content_type_override_string() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "text/plain")
		.body("csv accepted")
		.validate(|snap| {
			assert_eq!(snap.method, "POST");
			assert_eq!(snap.headers.get("content-type").map(|s| s.as_str()), Some("text/csv"));
			assert_eq!(snap.body, "a,b,c\n1,2,3");
		})
		.start()
		.await?;
	let url = server.path_url("/csv");

	// -- Exec
	let script =
		format!(r#"return aip.web.post{{ url = "{url}", body = "a,b,c\n1,2,3", content_type = "text/csv" }}"#,);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert_eq!(res.x_get_str("data")?, "csv accepted");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_api_web_post_omitted_body() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(WebModule)?.build()))?;
	crate::modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "text/plain")
		.body("empty body accepted")
		.validate(|snap| {
			assert_eq!(snap.method, "POST");
			assert!(snap.body.is_empty(), "Expected empty body");
		})
		.start()
		.await?;
	let url = server.path_url("/empty");

	// -- Exec
	let script = format!(r#"return aip.web.post{{ url = "{url}" }}"#);
	let res = _test_support::eval_script(&engine, &script).await?;

	// -- Check
	assert!(res.x_get_bool("success")?);
	assert_eq!(res.x_get_str("data")?, "empty body accepted");

	server.close().await?;

	Ok(())
}
