// region:    --- Modules

use super::gen_doc_impl::ModuleGroup;
use super::type_renderer::{render_type, render_type_block};
use crate::AipRegisteredFn;
use crate::schema_ref::SchemaRef;
use schemars::Schema;
use serde_json::Value;
use std::collections::HashMap;

// endregion: --- Modules

// region:    --- Fn Renderer

/// Format a multi-line docstring into TypeScript comments.
pub(crate) fn format_doc_comment(desc: &str) -> String {
	let trimmed_desc = desc.trim();
	if trimmed_desc.is_empty() {
		return String::new();
	}
	let mut s = String::new();
	for line in trimmed_desc.lines() {
		let trimmed = line.trim();
		if trimmed.is_empty() {
			s.push_str("//\n");
		} else {
			s.push_str(&format!("// {}\n", trimmed));
		}
	}
	s
}

/// Derive a PascalCase type name from a function path and a suffix (e.g., ("aip.file.list", "Params") -> "AipFileListParams").
#[allow(dead_code)]
pub(crate) fn path_to_type_name(path: &str, suffix: &str) -> String {
	let mut name = String::new();
	for part in path.split(['.', '_', '-']) {
		if !part.is_empty() {
			let mut chars = part.chars();
			if let Some(first) = chars.next() {
				for c in first.to_uppercase() {
					name.push(c);
				}
				name.push_str(chars.as_str());
			}
		}
	}
	name.push_str(suffix);
	name
}

/// Render a single function signature line with an optional comment description.
#[allow(dead_code)]
pub(crate) fn render_fn_signature(reg_fn: &AipRegisteredFn) -> String {
	let mut s = String::new();
	let desc: Option<String> = reg_fn
		.description
		.clone()
		.or_else(|| SchemaRef::new(&reg_fn.params_schema).desc().map(String::from));

	if let Some(d) = desc {
		s.push_str(&format_doc_comment(&d));
	}

	let params_type = if is_inlineable_type(&reg_fn.params_schema) {
		let t = render_type(&reg_fn.params_schema);
		if t == "any" || t == "{}" {
			path_to_type_name(&reg_fn.path, "Params")
		} else {
			t
		}
	} else {
		path_to_type_name(&reg_fn.path, "Params")
	};

	let output_type = if is_inlineable_type(&reg_fn.output_schema) {
		render_type(&reg_fn.output_schema)
	} else {
		path_to_type_name(&reg_fn.path, "Output")
	};

	s.push_str(&format!("{}(params: {}): {}\n", reg_fn.path, params_type, output_type));
	s
}

#[derive(Debug, Clone)]
pub(crate) struct ModuleTypeDefinition<'a> {
	pub name: String,
	pub schema: &'a Schema,
}

/// Render module signatures and collect deduplicated module-scoped types.
pub(crate) fn render_module<'a>(group: &ModuleGroup<'a>) -> (String, Vec<ModuleTypeDefinition<'a>>) {
	let mut seen_param_types: HashMap<String, String> = HashMap::new();
	let mut seen_output_types: HashMap<String, String> = HashMap::new();
	let mut module_types: Vec<ModuleTypeDefinition<'a>> = Vec::new();

	let mut signatures = Vec::new();

	for reg_fn in &group.fns {
		let desc: Option<String> = reg_fn
			.description
			.clone()
			.or_else(|| SchemaRef::new(&reg_fn.params_schema).desc().map(String::from));

		let comment = if let Some(d) = desc {
			format_doc_comment(&d)
		} else {
			String::new()
		};

		let params_type = if is_inlineable_type(&reg_fn.params_schema) {
			let t = render_type(&reg_fn.params_schema);
			if t == "any" || t == "{}" {
				path_to_type_name(&reg_fn.path, "Params")
			} else {
				t
			}
		} else {
			let key = serde_json::to_string(&reg_fn.params_schema).unwrap_or_default();
			if let Some(existing_name) = seen_param_types.get(&key) {
				existing_name.clone()
			} else {
				let name = path_to_type_name(&reg_fn.path, "Params");
				seen_param_types.insert(key, name.clone());
				module_types.push(ModuleTypeDefinition {
					name: name.clone(),
					schema: &reg_fn.params_schema,
				});
				name
			}
		};

		let output_type = if is_inlineable_type(&reg_fn.output_schema) {
			render_type(&reg_fn.output_schema)
		} else {
			let key = serde_json::to_string(&reg_fn.output_schema).unwrap_or_default();
			if let Some(existing_name) = seen_output_types.get(&key) {
				existing_name.clone()
			} else {
				let name = path_to_type_name(&reg_fn.path, "Output");
				seen_output_types.insert(key, name.clone());
				module_types.push(ModuleTypeDefinition {
					name: name.clone(),
					schema: &reg_fn.output_schema,
				});
				name
			}
		};

		signatures.push(format!(
			"{}{}(params: {}): {}\n",
			comment, reg_fn.path, params_type, output_type
		));
	}

	let mut sig_block = String::new();
	sig_block.push_str(&format!("## {}.*\n\n```ts\n", group.module_path));
	for (i, sig) in signatures.iter().enumerate() {
		if i > 0 {
			sig_block.push('\n');
		}
		sig_block.push_str(sig);
	}
	sig_block.push_str("```\n\n");

	(sig_block, module_types)
}

/// Render module-specific types under `### <module>.* Types`.
pub(crate) fn render_module_types(module_path: &str, types: &[ModuleTypeDefinition]) -> String {
	if types.is_empty() {
		return String::new();
	}
	let mut s = String::new();
	s.push_str(&format!("### {}.* Types\n\n```ts\n", module_path));
	for (i, t) in types.iter().enumerate() {
		if i > 0 {
			s.push('\n');
		}
		s.push_str(&render_type_block(t.schema, &t.name, false));
	}
	s.push_str("```\n\n");
	s
}

/// Render all function signatures for a module into a single TypeScript block.
#[allow(dead_code)]
pub(crate) fn render_module_signatures(module_path: &str, fns: &[&AipRegisteredFn]) -> String {
	let group = ModuleGroup {
		module_path: module_path.to_string(),
		fns: fns.to_vec(),
	};
	render_module(&group).0
}

/// Render a single registered function as a Markdown documentation section.
#[allow(dead_code)]
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
	if map.contains_key("properties") {
		return false;
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
