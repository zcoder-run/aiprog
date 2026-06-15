//! Tests for the aip.web module, following the same pattern as aip_json_tests.

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use crate::_test_support;
use crate::script::modules;

#[tokio::test]
async fn test_script_lua_web_constants() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_web::register)?;
	// Install the constants (must be done after the functions are installed)
	modules::aip_web::install_constants(&engine)?;

	// -- Exec
	let script = r#"
			local ua_aiprog = aip.web.UA_AIPROG
			local ua_browser = aip.web.UA_BROWSER
			return { ua_aiprog = ua_aiprog, ua_browser = ua_browser }
		"#;
	let res = _test_support::eval_script(&engine, script)?;

	// -- Check
	assert_eq!(res["ua_aiprog"], "aiprog");
	assert_eq!(
		res["ua_browser"],
		"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36"
	);

	Ok(())
}

#[tokio::test]
async fn test_script_lua_web_get_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_web::register)?;
	modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new()
		.status(200)
		.header("Content-Type", "application/json")
		.body(r#"{"hello":"world"}"#)
		.start()
		.await?;
	let url = server.path_url("/test");

	// -- Exec
	let lua = engine.lua();
	let script = format!(r#"return aip.web.get{{ data = "{url}", parse = true }}"#,);
	let func = lua.load(&script).into_function()?;
	let value: mlua::Value = func.call_async(()).await?;

	let table = value.as_table().ok_or("Expected table result")?;

	// -- Check
	assert!(table.get::<bool>("success")?);
	assert_eq!(table.get::<u16>("status")?, 200);
	let data: mlua::Value = table.get("data")?;
	let data_table = data.as_table().ok_or("Expected data table")?;
	assert_eq!(data_table.get::<String>("hello")?, "world");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_script_lua_web_post_json() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_web::register)?;
	modules::aip_web::install_constants(&engine)?;

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
	let lua = engine.lua();
	let script = format!(r#"return aip.web.post{{ data = "{url}", json = {{ key = "value" }}, parse = true }}"#,);
	let func = lua.load(&script).into_function()?;
	let value: mlua::Value = func.call_async(()).await?;

	let table = value.as_table().ok_or("Expected table result")?;

	// -- Check
	assert!(table.get::<bool>("success")?);
	assert_eq!(table.get::<u16>("status")?, 200);
	let data: mlua::Value = table.get("data")?;
	let data_table = data.as_table().ok_or("Expected data table")?;
	assert_eq!(data_table.get::<String>("result")?, "success");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_script_lua_web_post_body() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_web::register)?;
	modules::aip_web::install_constants(&engine)?;

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
	let lua = engine.lua();
	let script = format!(r#"return aip.web.post{{ data = "{url}", body = "hello world body" }}"#,);
	let func = lua.load(&script).into_function()?;
	let value: mlua::Value = func.call_async(()).await?;

	let table = value.as_table().ok_or("Expected table result")?;

	// -- Check
	assert!(table.get::<bool>("success")?);
	let data: String = table.get("data")?;
	assert_eq!(data, "received");

	server.close().await?;

	Ok(())
}

#[tokio::test]
async fn test_script_lua_web_post_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_web::register)?;
	modules::aip_web::install_constants(&engine)?;

	let server = _test_support::TestServerBuilder::new().start().await?;
	let url = server.path_url("/test");
	server.close().await?; // shut down the server to force a connection error

	// -- Exec
	let lua = engine.lua();
	let script = format!(r#"return aip.web.post{{ data = "{url}", json = {{ key = "value" }} }}"#,);
	let func = lua.load(&script).into_function()?;
	let result: mlua::Result<mlua::Value> = func.call_async(()).await;

	// -- Check
	assert!(result.is_err(), "Expected connection error but got Ok");

	Ok(())
}
