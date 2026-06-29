// region:    --- Modules

use crate::AipRegisteredFn;
use crate::Result;
use crate::ScriptEngine;
use crate::schema_ref::SchemaRef;
use schemars::Schema;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// endregion: --- Modules

// region:    --- Doc Generation

impl ScriptEngine {
	/// Generate documentation for all registered functions in Markdown format.
	///
	/// Each function is rendered as a section with its Lua path, description, signature,
	/// and parameter/output/error type definitions in TypeScript syntax.
	pub fn generate_doc(&self) -> Result<String> {
		let mut fns: Vec<&AipRegisteredFn> = self.registered_fns.iter().collect();
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
}

// endregion: --- Doc Generation

// region:    --- Renderer Functions

/// Render a single registered function as a Markdown documentation section.
fn render_fn(reg_fn: &AipRegisteredFn, error_schema: Option<&Schema>) -> String {
	let path = &reg_fn.path;
	let desc: Option<String> = reg_fn
		.description
		.clone()
		.or_else(|| SchemaRef::new(&reg_fn.params_schema).desc().map(String::from));

	let params_type = render_type_block(&reg_fn.params_schema, "Params", false);
	let output_type = render_type_block(&reg_fn.output_schema, "Output", false);
	let effective_error_schema = error_schema.unwrap_or(&reg_fn.error_schema);
	let error_type = render_type_block(effective_error_schema, "Error", true);

	let mut s = String::new();
	s.push_str(&format!("### {}\n\n", path));

	s.push_str(&format!("`{}(params: Params): Output`\n\n", path));

	if let Some(d) = desc {
		s.push_str(&format!("{}\n\n", d));
	}

	s.push_str("```ts\n");
	s.push_str(&params_type);
	s.push('\n');
	s.push_str(&output_type);
	s.push('\n');
	s.push_str(&error_type);
	s.push_str("```\n\n");
	s
}

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

/// Helper that renders a type alias block.
fn render_type_block(schema: &Schema, name: &str, include_desc: bool) -> String {
	let description = SchemaRef::new(schema).desc().map(String::from);
	let value = serde_json::to_value(schema).unwrap_or(Value::Null);
	let type_expr = render_value(&value);
	let mut s = String::new();
	if include_desc && let Some(desc) = description {
		for line in desc.lines() {
			s.push_str(&format!("// {}\n", line));
		}
	}
	s.push_str(&format!("type {} = {};\n", name, type_expr));
	s
}

#[allow(dead_code)]
/// Convert a `Schema` into a TypeScript-like type expression.
fn render_type(schema: &Schema) -> String {
	let value = serde_json::to_value(schema).unwrap_or(Value::Null);
	render_value(&value)
}

/// Core renderer that works on a JSON value representation of the schema.
fn render_value(v: &Value) -> String {
	match v {
		Value::Null => "any".to_string(),

		Value::Object(map) => {
			// Combinators that override the type field.
			if let Some(ref_val) = map.get("$ref") {
				let s = ref_val.as_str().unwrap_or("any");
				let last = s.rsplit('/').next().unwrap_or("any");
				return last.to_string();
			}
			if let Some(one_of) = map.get("oneOf") {
				return render_union(one_of);
			}
			if let Some(any_of) = map.get("anyOf") {
				return render_union(any_of);
			}
			if let Some(enum_val) = map.get("enum") {
				return render_enum(enum_val);
			}

			// Handle the "type" field.
			if let Some(typ) = map.get("type") {
				match typ {
					Value::String(s) => {
						if let Some(prim) = try_primitive_type_name(s.as_str()) {
							return prim.to_string();
						}
						match s.as_str() {
							"object" => return render_object_schema(map),
							"array" => return render_array(map),
							_ => {}
						}
					}
					Value::Array(type_list) => {
						let parts: Vec<String> = type_list
							.iter()
							.map(|t| {
								if let Some(s) = t.as_str() {
									if let Some(prim) = try_primitive_type_name(s) {
										prim.to_string()
									} else {
										render_value(t)
									}
								} else {
									render_value(t)
								}
							})
							.collect();
						return parts.join(" | ");
					}
					_ => {}
				}
			}

			// Fallback.
			"any".to_string()
		}
		_ => "any".to_string(),
	}
}

fn try_primitive_type_name(s: &str) -> Option<&str> {
	match s {
		"string" => Some("string"),
		"number" | "integer" => Some("number"),
		"boolean" => Some("boolean"),
		"null" => Some("null"),
		_ => None,
	}
}

fn render_array(map: &serde_json::Map<String, Value>) -> String {
	if let Some(items) = map.get("items") {
		let item_type = render_value(items);
		format!("Array<{}>", item_type)
	} else {
		"any[]".to_string()
	}
}

/// Given a type expression like "number | null" or "null | string",
/// remove the `null` alternative (and the separator) so that the
/// optional marker (`?`) alone conveys optionality.
/// Returns `"any"` if the type collapses to nothing.
fn simplify_optional_type(type_expr: &str) -> String {
	if !type_expr.contains(" | ") {
		if type_expr == "null" {
			return "any".to_string();
		}
		return type_expr.to_string();
	}

	let parts: Vec<&str> = type_expr.split(" | ").filter(|&p| p != "null").collect();

	if parts.is_empty() {
		"any".to_string()
	} else {
		parts.join(" | ")
	}
}

fn render_object_schema(map: &serde_json::Map<String, Value>) -> String {
	let schema = match Schema::try_from(Value::Object(map.clone())) {
		Ok(s) => s,
		Err(_) => return "{}".to_string(),
	};
	let schema_ref = SchemaRef::new(&schema);

	let mut props = schema_ref.properties();
	if props.is_empty() {
		return "{}".to_string();
	}
	props.sort_by(|a, b| match (a.is_required(), b.is_required()) {
		(true, false) => std::cmp::Ordering::Less,
		(false, true) => std::cmp::Ordering::Greater,
		_ => a.name().cmp(b.name()),
	});

	let mut out = String::new();
	out.push_str("{\n");

	for prop in props {
		if let Some(desc) = prop.desc() {
			for line in desc.lines() {
				out.push_str(&format!("  // {}\n", line));
			}
		}
		if let Some(default) = prop.default() {
			out.push_str(&format!("  // default: {}\n", default));
		}
		let mut type_expr = render_value(prop.raw_value());
		let optional_marker = if prop.is_required() {
			""
		} else {
			type_expr = simplify_optional_type(&type_expr);
			"?"
		};
		out.push_str(&format!("  {}{}: {};\n", prop.name(), optional_marker, type_expr));
	}

	out.push('}');
	out
}

fn render_union(union_val: &Value) -> String {
	if let Value::Array(items) = union_val {
		let types: Vec<String> = items.iter().map(render_value).collect();
		types.join(" | ")
	} else {
		"any".to_string()
	}
}

fn render_enum(enum_val: &Value) -> String {
	if let Value::Array(items) = enum_val {
		let parts: Vec<String> = items
			.iter()
			.map(|v| match v {
				Value::String(s) => format!("\"{}\"", s),
				other => format!("{}", other),
			})
			.collect();
		parts.join(" | ")
	} else {
		"any".to_string()
	}
}

// region:    --- Error Inline Helpers

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

// endregion: --- Error Inline Helpers

// endregion: --- Renderer Functions

// region:    --- Tests

#[cfg(test)]
#[path = "engine_doc_tests.rs"]
mod tests;

// endregion: --- Tests
