type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use crate::_test_support;
use crate::modules;
use assertables::{assert_contains, assert_not_contains};
use serde_json::json;

#[tokio::test]
async fn test_api_json_parse_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local content = '{"name": "John", "age": 30}'
            return aip.json.parse({ text = content })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert_eq!(res["name"], "John");
	assert_eq!(res["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_roundtrip() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local content = '{"name": "John", "age": 30}'
            local json_value = aip.json.parse({ text = content })
            return aip.json.stringify({ data = json_value })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	let data = res.as_str().ok_or("Expected string result")?;

	// -- Check
	assert_contains!(data, r#""age":30"#);

	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_with_comment() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local content = [[
						// Some comment
						{"name": "John", "age": 30}
					]]
            return aip.json.parse({ text = content })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert_eq!(res["name"], "John");
	assert_eq!(res["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_nil() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            return aip.json.parse({})
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert!(res.is_null());
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_invalid() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local ok, err = pcall(aip.json.parse, { text = "{invalid_json}" })
            if ok then
                return "should not reach here"
            else
                return tostring(err)
            end
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let err_str = res.as_str().ok_or("Expected error string")?;

	assert_contains!(err_str, "Fail to parse json");
	assert_contains!(err_str, "aip.json.parse failed");
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_jsonl_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local content = '{"name": "John", "age": 30}\n{"name": "Jane", "age": 25}'
            return aip.json.parse_jsonl({ text = content })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let expected = json!([
		{"name": "John", "age": 30},
		{"name": "Jane", "age": 25}
	]);
	assert_eq!(res, expected);
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_jsonl_empty_lines() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local content = '{"id": 1}\n\n{"id": 2}\n   \n{"id": 3}'
            return aip.json.parse_jsonl({ text = content })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let expected = json!([
		{"id": 1},
		{"id": 2},
		{"id": 3}
	]);
	assert_eq!(res, expected);
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_jsonl_nil() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            return aip.json.parse_jsonl({})
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert_eq!(res, json!({}));
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_jsonl_invalid_json() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local ok, err = pcall(aip.json.parse_jsonl, { text = '{"id": 1}\n{invalid_json}\n{"id": 3}' })
            if ok then
                return "should not reach here"
            else
                return tostring(err)
            end
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let err_str = res.as_str().ok_or("Expected error string")?;
	assert_contains!(err_str, "aip.json.parse_jsonl failed");
	assert_contains!(err_str, "line 2");
	Ok(())
}

#[tokio::test]
async fn test_api_json_stringify_pretty_basic() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local obj = {
                name = "John",
                age = 30
            }
            return aip.json.stringify({ data = obj, pretty = true })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	// -- Check
	let result = res.as_str().ok_or("Expected string result")?;
	let parsed: serde_json::Value = serde_json::from_str(result)?;
	assert_eq!(parsed["name"], "John");
	assert_eq!(parsed["age"], 30);
	assert!(result.contains('\n'), "Expected pretty formatting with newlines");
	assert!(result.contains("  "), "Expected pretty formatting with indentation");
	Ok(())
}

#[tokio::test]
async fn test_api_json_stringify_pretty_complex() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
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
            return aip.json.stringify({ data = obj, pretty = true })
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let result = res.as_str().ok_or("Expected string result")?;
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
async fn test_api_json_stringify_simple() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
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
	let res = _test_support::eval_script(&engine, script).await?;
	// -- Check
	let result = res.as_str().ok_or("Expected string result")?;
	assert_contains!(result, r#""name":"John""#);
	assert_not_contains!(result, "\n");
	assert_not_contains!(result, "  ");
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local res = aip.json.parse({ text = '{"name": "John", "age": 30}' })
            return res
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check standard API
	assert_eq!(res["name"], "John");
	assert_eq!(res["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_new_api_error() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local ok, res = pcall(aip.json.parse, { text = '{"invalid' })
            if not ok then
                return tostring(res)
            else
                return "should have failed"
            end
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check error message
	assert_contains!(res.as_str().ok_or("Expected error string")?, "Fail to parse json");
	Ok(())
}

#[tokio::test]
async fn test_api_json_parse_jsonl_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local res = aip.json.parse_jsonl({ text = '{"name": "John"}\n{"name": "Jane"}' })
            return res
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let expected = json!([
		{"name": "John"},
		{"name": "Jane"}
	]);
	assert_eq!(res, expected);
	Ok(())
}

#[tokio::test]
async fn test_api_json_stringify_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local res = aip.json.stringify({ data = { name = "John", age = 30 } })
            return res
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let result_str = res.as_str().ok_or("Expected string")?;
	assert_contains!(result_str, r#""name":"John""#);
	Ok(())
}

#[tokio::test]
async fn test_api_json_stringify_pretty_new_api() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            local res = aip.json.stringify({ data = { name = "John", age = 30 }, pretty = true })
            return res
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	let result_str = res.as_str().ok_or("Expected string")?;
	assert!(result_str.contains('\n'));
	assert!(result_str.contains("  "));
	Ok(())
}
#[tokio::test]
async fn test_api_json_stringify_to_line_alias_removed() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            return aip.json.stringify_to_line == nil
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;
	// -- Check
	assert!(res.as_bool().ok_or("Expected boolean result")?);
	Ok(())
}

#[tokio::test]
async fn test_api_json_chained_stringify_and_parse() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
        local obj = { name = "John", age = 30 }
        local stringified = aip.json.stringify({ data = obj })
        local parsed = aip.json.parse({ text = stringified })
        return parsed
    "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check (the round‑tripped object should match the JSON representation)
	assert_eq!(res["name"], "John");
	assert_eq!(res["age"], 30);
	Ok(())
}

#[tokio::test]
async fn test_api_json_stringify_nil_data() -> Result<()> {
	// -- Setup & Fixtures
	let engine = _test_support::setup_script_engine(modules::aip_json::init_registry)?;
	let script = r#"
            return aip.json.stringify({})
        "#;

	// -- Exec
	let res = _test_support::eval_script(&engine, script).await?;

	// -- Check
	assert!(res.is_null());
	Ok(())
}
