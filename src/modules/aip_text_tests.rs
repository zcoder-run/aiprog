type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use crate::_test_support;
use crate::AipModule as _;
use crate::AipRegistry;
use crate::AipRegistryBuilder;

fn aip_registry() -> crate::Result<AipRegistry> {
	let builder = AipRegistryBuilder::default();
	let reg = crate::modules::TextModule::register(builder)?.build();
	Ok(reg)
}

#[tokio::test]
async fn test_aip_text_format_size_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(aip_registry)?;
	let script = r#"
        return aip.text.format_size({ size = 777 })
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let s = res.as_str().ok_or("Expected string result")?;
	assert_eq!(s, "   777 B ");
	Ok(())
}

#[tokio::test]
async fn test_aip_text_format_size_nil() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(aip_registry)?;
	let script = r#"
        return aip.text.format_size({})
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert!(res.is_null(), "Expected nil return for nil size");
	Ok(())
}

#[tokio::test]
async fn test_aip_text_format_size_lowest_unit() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(aip_registry)?;
	let script = r#"
        return aip.text.format_size({ size = 1500, lowest_unit = "KB" })
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let s = res.as_str().ok_or("Expected string result")?;
	assert_eq!(s, "  1.50 KB");
	Ok(())
}

#[tokio::test]
async fn test_aip_text_format_size_trim() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(aip_registry)?;
	let script = r#"
        return aip.text.format_size({ size = 1500, lowest_unit = "KB", unpad = true })
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let s = res.as_str().ok_or("Expected string result")?;
	assert_eq!(s, "1.50 KB");
	Ok(())
}

#[tokio::test]
async fn test_aip_text_format_size_gb() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(aip_registry)?;
	let script = r#"
        return aip.text.format_size({ size = 2345678900 })
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let s = res.as_str().ok_or("Expected string result")?;
	assert_eq!(s, "  2.35 GB");
	Ok(())
}
