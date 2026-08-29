#![allow(non_snake_case)]

use super::fn_renderer::*;
use super::type_renderer::*;
use crate::aip_handler;
use crate::engine::support::LuaEngine;
use crate::impl_lua_serde_traits;
use crate::register_handler;
use crate::registry::HandlerResult;
use crate::{AipFnKind, AipOutput, AipParams, AipRegisteredFn, AipRegistryBuilder};
use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// -- Module grouping tests

#[test]
fn test_group_fns_by_module() {
	let params_schema = schema_for!(String);
	let output_schema = schema_for!(String);
	let error_schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let fn1 = AipRegisteredFn {
		path: "aip.file.list".to_string(),
		params_schema: params_schema.clone(),
		output_schema: output_schema.clone(),
		error_schema: error_schema.clone(),
		kind: AipFnKind::Sync,
		description: None,
		title: None,
	};
	let fn2 = AipRegisteredFn {
		path: "aip.file.read".to_string(),
		params_schema: params_schema.clone(),
		output_schema: output_schema.clone(),
		error_schema: error_schema.clone(),
		kind: AipFnKind::Sync,
		description: None,
		title: None,
	};
	let fn3 = AipRegisteredFn {
		path: "aip.time.now".to_string(),
		params_schema: params_schema.clone(),
		output_schema: output_schema.clone(),
		error_schema: error_schema.clone(),
		kind: AipFnKind::Sync,
		description: None,
		title: None,
	};

	let fns = vec![&fn3, &fn1, &fn2];
	let groups = super::gen_doc_impl::group_fns_by_module(&fns);

	assert_eq!(groups.len(), 2);
	assert_eq!(groups[0].module_path, "aip.file");
	assert_eq!(groups[0].fns.len(), 2);
	assert_eq!(groups[0].fns[0].path, "aip.file.list");
	assert_eq!(groups[0].fns[1].path, "aip.file.read");

	assert_eq!(groups[1].module_path, "aip.time");
	assert_eq!(groups[1].fns.len(), 1);
	assert_eq!(groups[1].fns[0].path, "aip.time.now");
}

#[test]
fn test_path_to_type_name() {
	assert_eq!(path_to_type_name("aip.file.list", "Params"), "AipFileListParams");
	assert_eq!(path_to_type_name("aip.time.offset", "Output"), "AipTimeOffsetOutput");
	assert_eq!(path_to_type_name("my_func", "Params"), "MyFuncParams");
	assert_eq!(
		path_to_type_name("custom-tool_name.do_work", "Params"),
		"CustomToolNameDoWorkParams"
	);
}

#[test]
fn test_render_fn_signature() {
	let params_schema = schema_for!(SimpleParams);
	let output_schema = schema_for!(String);
	let error_schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "aip.file.read".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("Reads a file from disk.".to_string()),
		title: None,
	};

	let sig = render_fn_signature(&reg_fn);
	assert!(sig.contains("// Reads a file from disk.\n"));
	assert!(sig.contains("aip.file.read(params: AipFileReadParams): string\n"));
}

#[test]
fn test_render_fn_signature_multiline() {
	let params_schema = schema_for!(SimpleParams);
	let output_schema = schema_for!(String);
	let error_schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "aip.file.read".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("First line of description.\n\nSecond line of description.".to_string()),
		title: None,
	};

	let sig = render_fn_signature(&reg_fn);
	assert!(sig.contains("// First line of description.\n//\n// Second line of description.\n"));
	assert!(sig.contains("aip.file.read(params: AipFileReadParams): string\n"));
}

type TestResult = core::result::Result<(), Box<dyn std::error::Error>>;

// -- Test schemas

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct SimpleParams {
	name: String,
	age: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ReadParams {
	path: String,
}

#[test]
fn test_render_module_signatures() {
	let params_schema1 = schema_for!(SimpleParams);
	let params_schema2 = schema_for!(ReadParams);
	let output_schema = schema_for!(String);
	let error_schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let fn1 = AipRegisteredFn {
		path: "aip.file.list".to_string(),
		params_schema: params_schema1,
		output_schema: output_schema.clone(),
		error_schema: error_schema.clone(),
		kind: AipFnKind::Sync,
		description: Some("Lists files matching glob patterns.".to_string()),
		title: None,
	};
	let fn2 = AipRegisteredFn {
		path: "aip.file.read".to_string(),
		params_schema: params_schema2,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("Reads a file from disk.".to_string()),
		title: None,
	};

	let fns = vec![&fn1, &fn2];
	let result = render_module_signatures("aip.file", &fns);

	assert!(result.starts_with("## aip.file.*\n\n```ts\n"));
	assert!(result.contains("// Lists files matching glob patterns.\n"));
	assert!(result.contains("aip.file.list(params: AipFileListParams): string\n"));
	assert!(result.contains("// Reads a file from disk.\n"));
	assert!(result.contains("aip.file.read(params: AipFileReadParams): string\n"));
	assert!(result.ends_with("```\n\n"));
}

#[test]
fn test_render_module_deduplication() {
	let params_schema = schema_for!(SimpleParams);
	let output_schema = schema_for!(String);
	let error_schema: Schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let fn1 = AipRegisteredFn {
		path: "aip.time.add".to_string(),
		params_schema: params_schema.clone(),
		output_schema: output_schema.clone(),
		error_schema: error_schema.clone(),
		kind: AipFnKind::Sync,
		description: Some("Adds time offset.".to_string()),
		title: None,
	};
	let fn2 = AipRegisteredFn {
		path: "aip.time.sub".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("Subtracts time offset.".to_string()),
		title: None,
	};

	let fns = vec![&fn1, &fn2];
	let group = super::gen_doc_impl::ModuleGroup {
		module_path: "aip.time".to_string(),
		fns,
	};
	let (sig_block, types) = render_module(&group);

	assert!(sig_block.contains("aip.time.add(params: AipTimeAddParams): string\n"));
	assert!(sig_block.contains("aip.time.sub(params: AipTimeAddParams): string\n"));
	assert_eq!(types.len(), 1);
	assert_eq!(types[0].name, "AipTimeAddParams");

	let types_block = render_module_types("aip.time", &types);
	assert!(types_block.contains("### aip.time.* Types\n\n```ts\n"));
	assert!(types_block.contains("type AipTimeAddParams = {"));
	assert!(!types_block.contains("AipTimeSubParams"));
}

// region:    --- Handler metadata tests

#[test]
fn test_render_fn_with_handler_description() {
	let params_schema = schema_for!(MultiLinePropDesc);
	let output_schema = schema_for!(String);
	let error_schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "my_func".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("Custom description".to_string()),
		title: None,
	};

	let result = render_fn(&reg_fn, None);
	assert!(result.contains("Custom description\n\n"));
}

#[test]
fn test_render_fn_with_handler_title() {
	let params_schema = schema_for!(MultiLinePropDesc);
	let output_schema = schema_for!(String);
	let error_schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "my_func".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: Some("Custom description".to_string()),
		title: Some("Custom Title".to_string()),
	};

	let result = render_fn(&reg_fn, None);
	assert!(!result.contains("Custom Title"));
	assert!(result.contains("Custom description\n\n"));
}

#[test]
fn test_render_fn_fallback_to_schema_description() {
	let params_schema = schema_for!(ParamsWithRootDesc);
	let output_schema = schema_for!(String);
	let error_schema = serde_json::from_value(serde_json::json!(true)).unwrap();

	let reg_fn = AipRegisteredFn {
		path: "my_func".to_string(),
		params_schema,
		output_schema,
		error_schema,
		kind: AipFnKind::Sync,
		description: None,
		title: None,
	};

	let result = render_fn(&reg_fn, None);
	assert!(result.contains("Root description for the params."));
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
		description: None,
		title: None,
	};

	let result = render_fn(&reg_fn, None);
	assert!(result.contains("### my_func\n"));
	// The root description should appear as function-level text, not as a comment
	assert!(!result.contains("// Root description for the params."));
	assert!(result.contains("Root description for the params."));
	assert!(result.contains("```ts\n"));
	assert!(result.contains("type Params = "));
	assert!(result.contains("params: Params): string"));
	assert!(!result.contains("type Output"));
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

	let lua = mlua::Lua::new();
	let engine = LuaEngine {
		lua,
		registered_fns: vec![AipRegisteredFn {
			path: "test.fn".to_string(),
			params_schema,
			output_schema,
			error_schema,
			kind: AipFnKind::Sync,
			description: None,
			title: None,
		}],
	};

	let doc = engine.generate_doc()?;
	assert!(doc.contains("## test.*"));
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
fn test_generate_doc_skips_common_error_block() -> TestResult {
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

	let lua = mlua::Lua::new();
	let engine = LuaEngine {
		lua,
		registered_fns: vec![AipRegisteredFn {
			path: "test.inline_error".to_string(),
			params_schema,
			output_schema,
			error_schema,
			kind: AipFnKind::Sync,
			description: None,
			title: None,
		}],
	};

	let doc = engine.generate_doc()?;

	// The common error type should appear only in the preamble, not per module.
	let error_type_count = doc.matches("type Error = {\n  message: string;\n};").count();
	assert_eq!(
		error_type_count, 1,
		"Common error type should appear only in preamble, not per-function; found {} occurrences:\n{}",
		error_type_count, doc
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

// region:    --- Integration tests for macro and registration

// -- Types for integration tests

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct DocSyncParams {
	name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct DocSyncOutput {
	greeting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct DocAsyncParams {
	value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct DocAsyncOutput {
	doubled: i64,
}

impl_lua_serde_traits!(DocSyncParams);
impl_lua_serde_traits!(DocSyncOutput);
impl_lua_serde_traits!(DocAsyncParams);
impl_lua_serde_traits!(DocAsyncOutput);
impl_lua_serde_traits!(ParamsWithRootDesc);

impl AipParams for DocSyncParams {}
impl AipOutput for DocSyncOutput {}
impl AipParams for DocAsyncParams {}
impl AipOutput for DocAsyncOutput {}
impl AipParams for ParamsWithRootDesc {}

// -- Handler definitions

/// # Sync handler title
///
/// Sync handler description text.
#[aip_handler]
#[allow(non_snake_case)]
fn DocSyncHandler(_call: crate::HandlerCallContext, params: DocSyncParams) -> HandlerResult<DocSyncOutput> {
	Ok(DocSyncOutput {
		greeting: format!("Hello, {}!", params.name),
	})
}

/// # Async handler title
///
/// Async handler description.
#[aip_handler]
#[allow(non_snake_case)]
async fn DocAsyncHandler(_call: crate::HandlerCallContext, params: DocAsyncParams) -> HandlerResult<DocAsyncOutput> {
	Ok(DocAsyncOutput {
		doubled: params.value * 2,
	})
}

/// # Multi-line handler title
///
/// First line of multi-line handler description.
///
/// Second paragraph with more details.
#[aip_handler]
#[allow(non_snake_case)]
fn DocMultiLineHandler(_call: crate::HandlerCallContext, params: DocSyncParams) -> HandlerResult<DocSyncOutput> {
	Ok(DocSyncOutput {
		greeting: format!("Hello, {}!", params.name),
	})
}

// -- Integration tests

#[test]
fn test_generate_doc_with_macro_sync_handler() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();
	register_handler!(registry, "aip.doc.sync", DocSyncHandler)?;
	let engine = crate::ScriptEngine::builder().with_registry(registry.build()).build()?;

	// -- Exec
	let doc = engine.generate_doc()?;

	// -- Check
	assert!(doc.contains("## aip.doc.*"));
	assert!(!doc.contains("Sync handler title"));
	assert!(doc.contains("Sync handler description text."));
	assert!(doc.contains("aip.doc.sync(params: AipDocSyncParams): AipDocSyncOutput"));
	assert!(doc.contains("### aip.doc.* Types"));
	assert!(doc.contains("type AipDocSyncParams = {"));
	assert!(doc.contains("name: string;"));
	assert!(doc.contains("type AipDocSyncOutput = {"));
	assert!(doc.contains("greeting: string;"));
	Ok(())
}

#[test]
fn test_generate_doc_with_macro_async_handler() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();
	register_handler!(registry, "aip.doc.async", DocAsyncHandler)?;
	let engine = crate::ScriptEngine::builder().with_registry(registry.build()).build()?;

	// -- Exec
	let doc = engine.generate_doc()?;

	// -- Check
	assert!(doc.contains("## aip.doc.*"));
	assert!(!doc.contains("Async handler title"));
	assert!(doc.contains("Async handler description."));
	assert!(doc.contains("aip.doc.async(params: AipDocAsyncParams): AipDocAsyncOutput"));
	assert!(doc.contains("### aip.doc.* Types"));
	assert!(doc.contains("type AipDocAsyncParams = {"));
	assert!(doc.contains("value: number;"));
	assert!(doc.contains("type AipDocAsyncOutput = {"));
	assert!(doc.contains("doubled: number;"));
	Ok(())
}

#[test]
fn test_generate_doc_with_macro_multiline_handler() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();
	register_handler!(registry, "aip.doc.multiline", DocMultiLineHandler)?;
	let engine = crate::ScriptEngine::builder().with_registry(registry.build()).build()?;

	// -- Exec
	let doc = engine.generate_doc()?;

	// -- Check
	assert!(doc.contains("## aip.doc.*"));
	assert!(!doc.contains("Multi-line handler title"));
	assert!(
		doc.contains("// First line of multi-line handler description.\n//\n// Second paragraph with more details.\n")
	);
	assert!(doc.contains("aip.doc.multiline(params: AipDocMultilineParams): AipDocMultilineOutput"));
	Ok(())
}

#[test]
fn test_generate_doc_fallback_to_params_desc() -> TestResult {
	// -- Setup & Fixtures
	fn fallback_impl(_call: crate::HandlerCallContext, params: ParamsWithRootDesc) -> HandlerResult<DocSyncOutput> {
		Ok(DocSyncOutput {
			greeting: format!("Hi, {}!", params.name),
		})
	}

	let registry = AipRegistryBuilder::default()
		.register_sync("aip.doc.fallback", fallback_impl)?
		.build();
	let engine = crate::ScriptEngine::builder().with_registry(registry).build()?;

	// -- Exec
	let doc = engine.generate_doc()?;

	// -- Check
	assert!(doc.contains("## aip.doc.*"));
	// Should fall back to Params schema description
	assert!(doc.contains("Root description for the params."));
	assert!(doc.contains("aip.doc.fallback(params: AipDocFallbackParams): AipDocFallbackOutput"));
	Ok(())
}

#[test]
fn test_generate_doc_includes_preamble() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();
	register_handler!(registry, "aip.doc.sync", DocSyncHandler)?;
	let engine = crate::ScriptEngine::builder().with_registry(registry.build()).build()?;

	// -- Exec
	let doc = engine.generate_doc()?;

	// -- Check
	assert!(doc.contains("# AIP Script Engine API"), "Preamble heading missing");
	assert!(
		doc.contains("## Function Signatures"),
		"Function Signatures section missing"
	);
	assert!(
		doc.contains("## Common Error Type"),
		"Common Error Type section missing"
	);
	// The common error type definition should appear in the preamble.
	assert!(
		doc.contains("type Error = {\n  message: string;\n};"),
		"Common error type definition missing from preamble"
	);
	Ok(())
}

#[test]
fn test_register_handler_duplicate_path() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();
	register_handler!(registry, "aip.dup.test", DocSyncHandler)?;

	// -- Exec
	let result = register_handler!(registry, "aip.dup.test", DocSyncHandler);

	// -- Check
	assert!(result.is_err());
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("Duplicate") || err.contains("already registered") || err.contains("already exists"),
		"Unexpected error message: {err}"
	);
	Ok(())
}

#[test]
fn test_register_handler_invalid_path() -> TestResult {
	// -- Setup & Fixtures
	let mut registry = AipRegistryBuilder::default();

	// -- Exec
	let result = register_handler!(registry, "", DocSyncHandler);

	// -- Check
	assert!(result.is_err());
	Ok(())
}

// endregion: --- Integration tests for macro and registration
