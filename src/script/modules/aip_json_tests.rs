type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use crate::_test_support::{eval_lua, setup_lua};
use crate::script::modules;
use assertables::{assert_contains, assert_not_contains};
use serde_json::json;

#[tokio::test]
async fn test_script_lua_json_parse_simple() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local content = '{"name": "John", "age": 30}'
            return aip.json.parse({ data = content })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	assert_eq!(res["data"]["name"], "John");
	assert_eq!(res["data"]["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_with_comment() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local content = [[
						// Some comment
						{"name": "John", "age": 30}
					]]
            return aip.json.parse({ data = content })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	assert_eq!(res["data"]["name"], "John");
	assert_eq!(res["data"]["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_nil() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            return aip.json.parse({})
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	assert!(res["data"].is_null());
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_invalid() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local ok, err = pcall(aip.json.parse, { data = "{invalid_json}" })
            if ok then
                return "should not reach here"
            else
                return tostring(err)
            end
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let err_str = res.as_str().ok_or("Expected error string")?;

	assert_contains!(err_str, "PARSE_FAILED");
	assert_contains!(&err_str, "json.parse failed");
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_ndjson_simple() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local content = '{"name": "John", "age": 30}\n{"name": "Jane", "age": 25}'
            return aip.json.parse_ndjson({ data = content })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let expected = json!([
		{"name": "John", "age": 30},
		{"name": "Jane", "age": 25}
	]);
	assert_eq!(res["data"], expected);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_ndjson_empty_lines() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local content = '{"id": 1}\n\n{"id": 2}\n   \n{"id": 3}'
            return aip.json.parse_ndjson({ data = content })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let expected = json!([
		{"id": 1},
		{"id": 2},
		{"id": 3}
	]);
	assert_eq!(res["data"], expected);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_ndjson_nil() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            return aip.json.parse_ndjson({})
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	assert_eq!(res["data"], json!([]));
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_ndjson_invalid_json() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local ok, err = pcall(aip.json.parse_ndjson, { data = '{"id": 1}\n{invalid_json}\n{"id": 3}' })
            if ok then
                return "should not reach here"
            else
                return tostring(err)
            end
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let err_str = res.as_str().ok_or("Expected error string")?;
	assert_contains!(err_str, "aip.json.parse_ndjson failed");
	assert_contains!(err_str, "line 2");
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_stringify_pretty_basic() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local obj = {
                name = "John",
                age = 30
            }
            return aip.json.stringify_pretty({ data = obj })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;
	// -- Check
	let result = res["data"].as_str().ok_or("Expected string result")?;
	let parsed: serde_json::Value = serde_json::from_str(result)?;
	assert_eq!(parsed["name"], "John");
	assert_eq!(parsed["age"], 30);
	assert!(result.contains('\n'), "Expected pretty formatting with newlines");
	assert!(result.contains("  "), "Expected pretty formatting with indentation");
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_stringify_pretty_complex() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local obj = {
                name = "John",
                age = 30,
                address = {
                    street = "123 Main St",
                    city = "New York"
                },
                hobbies = {"reading", "gaming"}
            }
            return aip.json.stringify_pretty({ data = obj })
        "#;

	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let result = res["data"].as_str().ok_or("Expected string result")?;
	let parsed: serde_json::Value = serde_json::from_str(result)?;
	assert_eq!(parsed["name"], "John");
	assert_eq!(parsed["age"], 30);
	assert_eq!(parsed["address"]["street"], "123 Main St");
	assert_eq!(parsed["hobbies"][0], "reading");
	assert!(result.contains('\n'), "Expected pretty formatting with newlines");
	assert!(result.contains("  "), "Expected pretty formatting with indentation");

	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_stringify_simple() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local obj = {
                name = "John",
                age = 30,
                address = {
                    street = "123 Main St",
                    city = "New York"
                },
                hobbies = {"reading", "gaming"}
            }
            return aip.json.stringify({ data = obj })
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;
	// -- Check
	let result = res["data"].as_str().ok_or("Expected string result")?;
	assert_contains!(result, r#""name":"John""#);
	assert_not_contains!(result, "\n");
	assert_not_contains!(result, "  ");
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local res = aip.json.parse({ data = '{"name": "John", "age": 30}' })
            return res
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check standard API
	assert_eq!(res["data"]["name"], "John");
	assert_eq!(res["data"]["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_new_api_error() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local ok, res = pcall(aip.json.parse, { data = '{"invalid' })
            if not ok then
                return tostring(res)
            else
                return "should have failed"
            end
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check error message
	assert_contains!(res.as_str().ok_or("Expected error string")?, "PARSE_FAILED");
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_parse_ndjson_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local res = aip.json.parse_ndjson({ data = '{"name": "John"}\n{"name": "Jane"}' })
            return res
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let expected = json!([
		{"name": "John"},
		{"name": "Jane"}
	]);
	assert_eq!(res["data"], expected);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_stringify_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local res = aip.json.stringify({ data = { name = "John", age = 30 } })
            return res
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let result_str = res["data"].as_str().ok_or("Expected string")?;
	assert_contains!(result_str, r#""name":"John""#);
	Ok(())
}

#[tokio::test]
async fn test_script_lua_json_stringify_pretty_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            local res = aip.json.stringify_pretty({ data = { name = "John", age = 30 } })
            return res
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;

	// -- Check
	let result_str = res["data"].as_str().ok_or("Expected string")?;
	assert!(result_str.contains('\n'));
	assert!(result_str.contains("  "));
	Ok(())
}
#[tokio::test]
async fn test_script_lua_json_stringify_to_line_alias_removed() -> Result<()> {
	// -- Setup & Fixtures
	let lua = setup_lua(modules::aip_json::init_module, "json").await?;
	let script = r#"
            return aip.json.stringify_to_line == nil
        "#;
	// -- Exec
	let res = eval_lua(&lua, script)?;
	// -- Check
	assert!(res.as_bool().ok_or("Expected boolean result")?);
	Ok(())
}
