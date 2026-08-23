// region:    --- Modules

use super::type_renderer::{render_type, render_type_block};
use crate::AipRegisteredFn;
use crate::schema_ref::SchemaRef;
use schemars::Schema;
use serde_json::Value;

// endregion: --- Modules

// region:    --- Fn Renderer

/// Render a single registered function as a Markdown documentation section.
pub(crate) fn render_fn(reg_fn: &AipRegisteredFn, error_schema: Option<&Schema>) -> String {
	let path = &reg_fn.path;
	let desc: Option<String> = reg_fn
		.description
		.clone()
		.or_else(|| SchemaRef::new(&reg_fn.params_schema).desc().map(String::from));

	let params_type = render_type_block(&reg_fn.params_schema, "Params", false);
	let output_inlineable = is_inlineable_type(&reg_fn.output_schema);

	let mut s = String::new();
	s.push_str(&format!("### {}\n\n", path));

	if output_inlineable {
		let output_expr = render_type(&reg_fn.output_schema);
		s.push_str(&format!("`{}(params: Params): {}`\n\n", path, output_expr));
	} else {
		s.push_str(&format!("`{}(params: Params): Output`\n\n", path));
	}

	if let Some(d) = desc {
		s.push_str(&format!("{}\n\n", d));
	}

	s.push_str("```ts\n");
	s.push_str(&params_type);
	s.push('\n');
	if !output_inlineable {
		let output_type = render_type_block(&reg_fn.output_schema, "Output", false);
		s.push_str(&output_type);
		s.push('\n');
	}
	if error_schema.is_none() {
		let error_type = render_type_block(&reg_fn.error_schema, "Error", true);
		s.push_str(&error_type);
	}
	s.push_str("```\n\n");
	s
}

/// Returns true if the schema renders to a simple type expression that can be inlined
/// directly in the function signature (primitives, refs, arrays, enums, unions).
/// Objects are not inlineable and remain expanded in a type block.
pub(crate) fn is_inlineable_type(schema: &Schema) -> bool {
	let value = serde_json::to_value(schema).unwrap_or(Value::Null);
	let Some(map) = value.as_object() else {
		return true;
	};

	if map.contains_key("$ref") {
		return true;
	}
	if map.contains_key("oneOf") || map.contains_key("anyOf") {
		return true;
	}
	if map.contains_key("enum") {
		return true;
	}

	if let Some(typ) = map.get("type") {
		match typ {
			Value::String(s) => return s != "object",
			Value::Array(_) => return true,
			_ => {}
		}
	}

	true
}

// endregion: --- Fn Renderer
