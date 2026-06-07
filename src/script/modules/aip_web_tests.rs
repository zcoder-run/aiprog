//! Tests for the aip.web module, following the same pattern as aip_json_tests.

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use crate::_test_support;
use crate::script::modules;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

	// Start a local test server
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	let url = format!("http://{addr}/test");

	let server_task = tokio::spawn(async move {
		let (mut socket, _) = listener.accept().await?;
		let mut buf = [0u8; 1024];
		let _n = socket.read(&mut buf).await?;
		let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\nconnection: close\r\n\r\n{\"hello\":\"world\"}";
		socket.write_all(response).await?;
		socket.shutdown().await
	});

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

	server_task.await??;

	Ok(())
}
