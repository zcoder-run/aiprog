use super::*;
use crate::{AipFnKind, AipRegistry};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::json;

type TestResult = core::result::Result<(), Box<dyn std::error::Error>>;

// -- Test schemas

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct SimpleParams {
	name: String,
	age: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ObjectWithDesc {
	/// A description on the name field.
	#[schemars(description = "A description on the name field.")]
	name: String,
}

/// Root description for the params.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ParamsWithRootDesc {
	name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ObjectWithRequired {
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	age: Option<i64>,
}

/// First line.
/// Second line.
/// Third line.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct MultiLineRootDesc {
	value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct MultiLinePropDesc {
	#[schemars(description = "First line.\nSecond line.\nThird line.")]
	prop: String,
}

// -- Primitives

#[test]
fn test_render_type_string() {
	let schema = schema_for!(String);
	let result = render_type(&schema);
	assert_eq!(result, "string");
}

#[test]
fn test_render_type_number() {
	let schema = schema_for!(i32);
	let result = render_type(&schema);
	assert_eq!(result, "number");
}

#[test]
fn test_render_type_boolean() {
	let schema = schema_for!(bool);
	let result = render_type(&schema);
	assert_eq!(result, "boolean");
}

#[test]
fn test_render_type_null_schema() {
	let schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();
	let result = render_type(&schema);
	assert_eq!(result, "any");
}

// -- Objects

#[test]
fn test_render_type_object_simple() {
	let schema = schema_for!(SimpleParams);
	let result = render_type(&schema);
	assert!(result.contains("name: string;"));
	assert!(result.contains("age?: number;"));
}

#[test]
fn test_render_type_object_with_description() {
	let schema = schema_for!(ObjectWithDesc);
	let result = render_type(&schema);
	// Ensure comment appears exactly once.
	assert_eq!(
        result.matches("  // A description on the name field.").count(),
		1,
		"Expected exactly one comment; got:\n{}",
		result
	);
}

#[test]
fn test_render_type_object_with_required() {
	let schema = schema_for!(ObjectWithRequired);
	let result = render_type(&schema);
	// name is required (no `?`), age is optional.
	assert!(result.contains("name: string;"));
	assert!(result.contains("age?: number;"));
	// Verify required property appears before optional property.
	let name_pos = result.find("name:").expect("name field missing");
	let age_pos = result.find("age?:").expect("age field missing");
	assert!(
		name_pos < age_pos,
		"Required field 'name' should appear before optional 'age'"
	);
}

// -- Array

#[test]
fn test_render_type_array() {
	let schema = schema_for!(Vec<String>);
	let result = render_type(&schema);
	assert_eq!(result, "Array<string>");
}

// -- Enums / unions

#[test]
fn test_render_type_enum_strings() {
	// Create a JSON schema with enum.
	let mut map = serde_json::Map::new();
	map.insert("type".into(), "string".into());
	map.insert("enum".into(), Value::Array(vec!["foo".into(), "bar".into()]));
	let value = Value::Object(map);
	let result = render_value(&value);
	assert_eq!(result, "\"foo\" | \"bar\"");
}

#[test]
fn test_render_type_one_of() {
	let mut map = serde_json::Map::new();
	map.insert(
		"oneOf".into(),
		Value::Array(vec![
			{
				let mut m = serde_json::Map::new();
				m.insert("type".into(), "string".into());
				Value::Object(m)
			},
			{
				let mut m = serde_json::Map::new();
				m.insert("type".into(), "number".into());
				Value::Object(m)
			},
		]),
	);
	let value = Value::Object(map);
	let result = render_value(&value);
	assert!(result.contains("string | number"));
}

// -- render_fn

#[test]
fn test_render_fn_basic() {
	let params_schema = schema_for!(ParamsWithRootDesc);
	let output_schema = schema_for!(String);
	let error_schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "my_func".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
	};

	let result = render_fn(&reg_fn, None);
	assert!(result.contains("### my_func\n"));
	// The root description should appear as function-level text, not as a comment
	assert!(!result.contains("// Root description for the params."));
	assert!(result.contains("Root description for the params."));
	assert!(result.contains("Signature: `my_func(params: Params): Output`"));
	assert!(result.contains("```ts\n"));
	assert!(result.contains("type Params = "));
	assert!(result.contains("type Output = string;\n"));
	assert!(result.contains("type Error = any;\n"));
	assert!(result.contains("```\n"));
}
// -- Multi-line descriptions

#[test]
fn test_render_type_block_multi_line_root_description() {
	let schema = schema_for!(MultiLineRootDesc);
    let result = render_type_block(&schema, "Params", true);
	// Multi-line root descriptions now use // single-line comments.
	assert!(result.contains("// First line.\n"));
	assert!(result.contains("// Second line.\n"));
	assert!(result.contains("// Third line.\n"));
	assert!(!result.contains("/*"));
}

#[test]
fn test_render_value_multi_line_property_description() {
	let schema = schema_for!(MultiLinePropDesc);
	let result = render_value(&serde_json::to_value(&schema).unwrap());
	// Property comment should contain each line prefixed
    assert!(result.contains("  // First line.\n"));
    assert!(result.contains("  // Second line.\n"));
    assert!(result.contains("  // Third line.\n"));
}

// -- Ref simplification

#[test]
fn test_render_value_ref_simplification() {
	let mut map = serde_json::Map::new();
	map.insert("$ref".into(), "#/$defs/FileGlobs".into());
	let value = Value::Object(map);
	let result = render_value(&value);
	assert_eq!(result, "FileGlobs");
}

#[test]
fn test_render_value_ref_no_prefix() {
	// Already a simple type name.
	let mut map = serde_json::Map::new();
	map.insert("$ref".into(), "FileGlobs".into());
	let value = Value::Object(map);
	let result = render_value(&value);
	assert_eq!(result, "FileGlobs");
}

#[test]
fn test_render_value_ref_nested() {
	// With nested path.
	let mut map = serde_json::Map::new();
	map.insert("$ref".into(), "#/$defs/SomeNested/Type".into());
	let value = Value::Object(map);
	let result = render_value(&value);
	assert_eq!(result, "Type");
}

#[test]
fn test_generate_doc_shared_types() -> TestResult {
	let params_schema: Schema = Schema::try_from(json!({
		"type": "object",
		"properties": {
			"config": { "$ref": "#/$defs/SharedConfig" }
		},
		"required": ["config"],
		"$defs": {
			"SharedConfig": {
				"description": "A shared configuration object",
				"type": "object",
				"properties": {
					"port": { "type": "integer", "default": 8080 }
				}
			}
		}
	}))
	.expect("Invalid schema");
	let output_schema: Schema = Schema::try_from(json!(true)).expect("Invalid schema");
	let error_schema: Schema = Schema::try_from(json!({"type": "string"})).expect("Invalid schema");

	let mut engine = ScriptEngine::from_registry(AipRegistry::from_empty())?;
	engine.registered_fns.push(AipRegisteredFn {
		path: "test.fn".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
	});

	let doc = engine.generate_doc()?;
	assert!(doc.contains("### test.fn"));
	assert!(doc.contains("## Shared Types"));
	assert!(doc.contains("type SharedConfig"));
	assert!(doc.contains("// A shared configuration object"));
	assert!(
        doc.contains("// default: 8080"),
        "Output should contain default comment for port"
	);
	Ok(())
}

#[test]
fn test_generate_doc_inline_kind_none_error() -> TestResult {
	let params_schema: Schema = Schema::try_from(json!({"type": "string"})).expect("Invalid schema");
	let output_schema: Schema = Schema::try_from(json!({"type": "number"})).expect("Invalid schema");
	let error_schema: Schema = Schema::try_from(json!({
		"type": "object",
		"properties": {
			"kind": { "$ref": "#/$defs/KindNone" },
			"message": { "type": "string" }
		},
		"required": ["kind", "message"],
		"$defs": {
			"KindNone": {
				"type": "object",
				"properties": {}
			}
		}
	}))
	.expect("Invalid schema");

	let mut engine = ScriptEngine::from_registry(AipRegistry::from_empty())?;
	engine.registered_fns.push(AipRegisteredFn {
		path: "test.inline_error".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
	});

	let doc = engine.generate_doc()?;

	let expected_error = "type Error = {\n  message: string;\n};";
	assert!(
		doc.contains(expected_error),
		"Expected inlined error type, got:\n{}",
		doc
	);
	assert!(!doc.contains("KindNone"), "KindNone should not be in output:\n{}", doc);
	Ok(())
}

#[test]
fn test_render_type_block_include_desc_false() {
    let schema = schema_for!(ParamsWithRootDesc);
    let result = render_type_block(&schema, "Params", false);
    assert!(!result.contains("// Root description"));
    assert!(result.contains("type Params = {"));
}
