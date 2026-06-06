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
