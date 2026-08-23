// region:    --- Modules

use super::fn_renderer::render_fn;
use super::type_renderer::render_type_block;
use crate::AipRegisteredFn;
use crate::Result;
use crate::engine::LuaEngine;
use crate::schema_ref::SchemaRef;
use schemars::Schema;
use std::collections::{HashMap, HashSet};

// endregion: --- Modules

// region:    --- Doc Generation

/// Generate documentation for the given list of registered functions in Markdown format.
///
/// Each function is rendered as a section with its Lua path, description, signature,
/// and parameter/output/error type definitions in TypeScript syntax.
///
/// Use this with a slice of [`AipRegisteredFn`] obtained from
/// [`AipRegistry::list_registered_fns`](crate::AipRegistry::list_registered_fns).
pub fn generate_doc_from_fns(fns: &[AipRegisteredFn]) -> Result<String> {
	let mut fns: Vec<&AipRegisteredFn> = fns.iter().collect();
	fns.sort_by(|a, b| a.path.cmp(&b.path));

	// Detect error schemas that are HandlerError<KindNone> and should be inlined.
	let use_inline_error: Vec<bool> = fns.iter().map(|f| is_kind_none_error_schema(&f.error_schema)).collect();

	let inline_error_schema = inline_error_schema();

	let mut all_ref_keys = HashSet::new();
	let mut definitions: HashMap<String, Schema> = HashMap::new();

	for (i, reg_fn) in fns.iter().enumerate() {
		collect_schema_info(&reg_fn.params_schema, &mut all_ref_keys, &mut definitions);
		collect_schema_info(&reg_fn.output_schema, &mut all_ref_keys, &mut definitions);
		if !use_inline_error[i] {
			collect_schema_info(&reg_fn.error_schema, &mut all_ref_keys, &mut definitions);
		}
	}

	let mut doc = String::new();
	doc.push_str(include_str!("engine_doc_preamble.md"));
	// TODO: Should have a support::ensure_end_with_two_nline
	doc.push_str("\n\n");

	for (i, reg_fn) in fns.iter().enumerate() {
		let error_schema = if use_inline_error[i] {
			Some(&inline_error_schema)
		} else {
			None
		};
		doc.push_str(&render_fn(reg_fn, error_schema));
	}

	if !all_ref_keys.is_empty() {
		let mut sorted_keys: Vec<&String> = all_ref_keys.iter().collect();
		sorted_keys.sort();
		doc.push_str("## Shared Types\n\n```ts\n");
		for key in sorted_keys {
			if let Some(def_schema) = definitions.get(key) {
				doc.push_str(&render_type_block(def_schema, key, true));
				doc.push('\n');
			}
		}
		doc.push_str("```\n");
	}

	Ok(doc)
}

impl LuaEngine {
	pub fn generate_doc(&self) -> Result<String> {
		generate_doc_from_fns(&self.registered_fns)
	}
}

// endregion: --- Doc Generation

// region:    --- Schema Helpers

/// Collect `$defs` keys and definitions from a schema for shared type generation.
fn collect_schema_info(schema: &Schema, ref_keys: &mut HashSet<String>, definitions: &mut HashMap<String, Schema>) {
	let schema_ref = SchemaRef::new(schema);
	for key in schema_ref.ref_keys() {
		ref_keys.insert(key.to_string());
	}
	if let Some(obj) = schema.as_value().as_object()
		&& let Some(defs) = obj.get("$defs").and_then(|v| v.as_object())
	{
		for (k, v) in defs {
			if let Ok(s) = Schema::try_from(v.clone()) {
				definitions.entry(k.clone()).or_insert_with(|| s);
			}
		}
	}
}

/// Returns true if the schema represents a `HandlerError<KindNone>`.
fn is_kind_none_error_schema(schema: &Schema) -> bool {
	if let Some(obj) = schema.as_value().as_object()
		&& let Some(props) = obj.get("properties").and_then(|v| v.as_object())
		&& let Some(kind) = props.get("kind")
		&& let Some(kind_obj) = kind.as_object()
		&& let Some(ref_val) = kind_obj.get("$ref")
		&& let Some(s) = ref_val.as_str()
	{
		s.contains("KindNone")
	} else {
		false
	}
}

/// Create an inlined error schema equivalent to `HandlerError<KindNone>`.
fn inline_error_schema() -> Schema {
	Schema::try_from(serde_json::json!({
		"type": "object",
		"properties": {
			"message": { "type": "string" }
		},
		"required": ["message"]
	}))
	.expect("Failed to create inline error schema")
}

// endregion: --- Schema Helpers
