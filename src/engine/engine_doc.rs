// region:    --- Modules

use crate::AipRegisteredFn;
use crate::Result;
use crate::ScriptEngine;
use schemars::Schema;
use serde_json::Value;

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
		let mut doc = String::new();
		for reg_fn in fns {
			doc.push_str(&render_fn(reg_fn));
		}
		Ok(doc)
	}
}

// endregion: --- Doc Generation

// region:    --- Renderer Functions

/// Render a single registered function as a Markdown documentation section.
fn render_fn(reg_fn: &AipRegisteredFn) -> String {
	let path = &reg_fn.path;
	let desc = get_root_description(&reg_fn.params_schema);
	let params_type = render_params(&reg_fn.params_schema);
	let output_type = render_output(&reg_fn.output_schema);
	let error_type = render_error(&reg_fn.error_schema);

	let mut s = String::new();
	s.push_str(&format!("### {}\n\n", path));
	if let Some(d) = desc {
		s.push_str(&format!("{}\n\n", d));
	}
	s.push_str(&format!("Signature: `{}(params: Params): Output`\n\n", path));
	s.push_str("```ts\n");
	s.push_str(&params_type);
	s.push('\n');
	s.push_str(&output_type);
	s.push('\n');
	s.push_str(&error_type);
	s.push_str("```\n\n");
	s
}

fn render_params(schema: &Schema) -> String {
	render_type_block(schema, "Params")
}

fn render_output(schema: &Schema) -> String {
	render_type_block(schema, "Output")
}

fn render_error(schema: &Schema) -> String {
	render_type_block(schema, "Error")
}

/// Helper that renders a type alias block.
fn render_type_block(schema: &Schema, name: &str) -> String {
	let mut value = serde_json::to_value(schema).unwrap_or(Value::Null);
	let description = value
		.as_object_mut()
		.and_then(|map| map.remove("description"))
		.and_then(|v| v.as_str().map(|s| s.to_string()));
	let type_expr = render_value(&value);
	let mut s = String::new();
	if let Some(desc) = description {
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
				return ref_val.as_str().unwrap_or("any").to_string();
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
							"object" => return render_schema_object(map),
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

fn render_schema_object(map: &serde_json::Map<String, Value>) -> String {
	let properties = match map.get("properties") {
		Some(Value::Object(props)) => props,
		_ => return "{}".to_string(),
	};

	let required: Vec<&str> = match map.get("required") {
		Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
		_ => vec![],
	};

	let mut out = String::new();
	out.push_str("{\n");

	let mut keys: Vec<&String> = properties.keys().collect();
	keys.sort();

	for key in keys {
		let prop_val = &properties[key];
		if let Value::Object(prop_map) = prop_val {
			// Emit description comment.
			if let Some(desc) = prop_map.get("description").and_then(|v| v.as_str()) {
				for line in desc.lines() {
					out.push_str(&format!("  // {}\n", line));
				}
			}

			// Emit default comment.
			if let Some(default) = prop_map.get("default") {
				out.push_str(&format!("  // default: {}\n", default));
			}

			// Clone and strip metadata before type rendering to avoid duplication.
			let mut clean_prop = prop_val.clone();
			if let Value::Object(ref mut m) = clean_prop {
				m.remove("description");
				m.remove("default");
			}

			let mut type_expr = render_value(&clean_prop);
			let optional_marker = if required.contains(&key.as_str()) {
				""
			} else {
				type_expr = simplify_optional_type(&type_expr);
				"?"
			};
			out.push_str(&format!("  {}{}: {};\n", key, optional_marker, type_expr));
		} else {
			out.push_str(&format!("  {}: any;\n", key));
		}
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

fn get_root_description(schema: &Schema) -> Option<String> {
	let value = serde_json::to_value(schema).ok()?;
	value.as_object()?.get("description")?.as_str().map(|s| s.to_string())
}

// endregion: --- Renderer Functions

// region:    --- Tests

#[cfg(test)]
#[path = "engine_doc_tests.rs"]
mod tests;

// endregion: --- Tests
