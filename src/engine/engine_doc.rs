use crate::Result;
use crate::{AipFnKind, ScriptEngine};

impl ScriptEngine {
	/// Generate Markdown documentation for all registered functions.
	///
	/// Each function is documented with its AIP path, kind (sync/async),
	/// and JSON schemas for parameters, response, and error types.
	pub fn generate_doc(&self) -> Result<String> {
		let doc = self.generate_doc_inner(true)?;
		Ok(doc)
	}
}

impl ScriptEngine {
	fn generate_doc_inner(&self, heading_only: bool) -> Result<String> {
		let mut doc = String::new();

		for fn_meta in &self.registered_fns {
			let kind_str = match fn_meta.kind {
				AipFnKind::Sync => "sync",
				AipFnKind::Async => "async",
			};

			// Level-2 heading with path and kind
			doc.push_str(&format!("## `{}` ({})\n\n", fn_meta.path, kind_str));

			if !heading_only {
				// Parameters schema
				doc.push_str("### Parameters schema\n\n");
				doc.push_str("```json\n");
				let json_str = serde_json::to_string_pretty(&fn_meta.params_schema)
					.map_err(|e| crate::Error::cc("Failed to serialize parameters schema", e))?;
				doc.push_str(&json_str);
				if !json_str.ends_with('\n') {
					doc.push('\n');
				}
				doc.push_str("```\n\n");

				// Response schema
				doc.push_str("### Response schema\n\n");
				doc.push_str("```json\n");
				let json_str = serde_json::to_string_pretty(&fn_meta.output_schema)
					.map_err(|e| crate::Error::cc("Failed to serialize response schema", e))?;
				doc.push_str(&json_str);
				if !json_str.ends_with('\n') {
					doc.push('\n');
				}
				doc.push_str("```\n\n");

				// Error schema
				doc.push_str("### Error schema\n\n");
				doc.push_str("```json\n");
				let json_str = serde_json::to_string_pretty(&fn_meta.error_schema)
					.map_err(|e| crate::Error::cc("Failed to serialize error schema", e))?;
				doc.push_str(&json_str);
				if !json_str.ends_with('\n') {
					doc.push('\n');
				}
				doc.push_str("```\n\n");
			}
		}

		Ok(doc)
	}
}

// region:    --- Support

// endregion: --- Support
