type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

use crate::_test_support;
use crate::modules;

#[tokio::test]
async fn test_aip_md_make_table_basic() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(modules::aip_md::init_registry)?;
	let script = r#"
		local headers = { "Name", "Age", "City" }
		local rows = {
			{ "Alice", 30, "New York" },
			{ "Bob", 25, "San Francisco" }
		}
		return aip.md.make_table({ headers = headers, rows = rows })
	"#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	let table_str = res.as_str().ok_or("Expected string output")?;

	// -- Check
	let expected = "\
| Name  | Age | City          |
| ----- | --- | ------------- |
| Alice | 30  | New York      |
| Bob   | 25  | San Francisco |";
	assert_eq!(table_str, expected);
	Ok(())
}

#[tokio::test]
async fn test_aip_md_make_table_with_types_and_null() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(modules::aip_md::init_registry)?;
	let script = r#"
		local headers = { "Item", "Score", "Active", "Notes" }
		local rows = {
			{ "Alpha", 98.5, true, nil },
			{ "Beta", 42, false, "Done" }
		}
		return aip.md.make_table({ headers = headers, rows = rows })
	"#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	let table_str = res.as_str().ok_or("Expected string output")?;

	// -- Check
	let expected = "\
| Item  | Score | Active | Notes |
| ----- | ----- | ------ | ----- |
| Alpha | 98.5  | true   |       |
| Beta  | 42    | false  | Done  |";
	assert_eq!(table_str, expected);
	Ok(())
}

#[tokio::test]
async fn test_aip_md_make_table_no_headers() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(modules::aip_md::init_registry)?;
	let script = r#"
		local rows = {
			{ "Col1", "Col2" },
			{ "Val1", "Val2" }
		}
		return aip.md.make_table({ rows = rows })
	"#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	let table_str = res.as_str().ok_or("Expected string output")?;

	// -- Check
	let expected = "\
| Col1 | Col2 |
| Val1 | Val2 |";
	assert_eq!(table_str, expected);
	Ok(())
}

#[tokio::test]
async fn test_aip_md_make_table_missing_rows_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_lua_engine(modules::aip_md::init_registry)?;
	let script = r#"
		local ok, err = pcall(aip.md.make_table, { headers = { "Col" } })
		if ok then
			return "should have failed"
		else
			return tostring(err)
		end
	"#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	let err_str = res.as_str().ok_or("Expected error string")?;

	// -- Check
	assert!(err_str.contains("Missing required property 'rows'"));
	Ok(())
}
