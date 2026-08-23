use crate::schema_ref::SchemaRef;
use schemars::Schema;
use serde_json::Value;

// region:    --- Type Renderer

/// Helper that renders a type alias block.
pub(crate) fn render_type_block(schema: &Schema, name: &str, include_desc: bool) -> String {
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

/// Convert a `Schema` into a TypeScript-like type expression.
pub(crate) fn render_type(schema: &Schema) -> String {
	let value = serde_json::to_value(schema).unwrap_or(Value::Null);
	render_value(&value)
}

/// Core renderer that works on a JSON value representation of the schema.
pub(crate) fn render_value(v: &Value) -> String {
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

// endregion: --- Type Renderer
