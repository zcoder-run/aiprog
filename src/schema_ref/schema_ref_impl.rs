use schemars::Schema;
use serde_json::Value;
use value_ext::JsonValueExt;

// region:    --- SchemaRef

pub struct SchemaRef<'s> {
	schema: &'s Schema,
}

impl<'s> SchemaRef<'s> {
	pub fn new(schema: &'s Schema) -> Self {
		SchemaRef { schema }
	}

	/// Returns the root-level `description` field, if present.
	pub fn desc(&self) -> Option<&'s str> {
		self.schema.as_value().x_get_str("description").ok()
	}

	/// Returns the `type` field of the schema, if present.
	pub fn typ(&self) -> Option<&'s str> {
		self.schema.as_value().x_get_str("type").ok()
	}

	/// Returns the properties of the object schema as `SchemaPropRef` wrappers,
	/// each annotated with its required status based on the `required` array.
	pub fn properties(&self) -> Vec<SchemaPropRef<'s>> {
		let value = self.schema.as_value();

		let properties = value
			.as_object()
			.and_then(|obj| obj.get("properties"))
			.and_then(|v| v.as_object());
		let properties = match properties {
			Some(props) => props,
			None => return vec![],
		};

		let required: Vec<&str> = value
			.x_get_as::<&Vec<Value>>("required")
			.ok()
			.map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
			.unwrap_or_default();

		properties
			.iter()
			.map(|(name, prop_value)| {
				let required = required.contains(&name.as_str());
				SchemaPropRef {
					name: name.as_str(),
					value: prop_value,
					required,
				}
			})
			.collect()
	}

	/// Returns the underlying JSON value of the schema.
	pub fn raw_value(&self) -> &'s Value {
		self.schema.as_value()
	}
}

// endregion: --- SchemaRef

// region:    --- SchemaPropRef

pub struct SchemaPropRef<'s> {
	name: &'s str,
	value: &'s Value,
	required: bool,
}

impl<'s> SchemaPropRef<'s> {
	pub fn name(&self) -> &str {
		self.name
	}

	/// Returns the `description` field of the property, if present.
	pub fn desc(&self) -> Option<&str> {
		self.value.x_get_str("description").ok()
	}

	/// Returns the `type` field of the property, if present.
	pub fn typ(&self) -> Option<&str> {
		self.value.x_get_str("type").ok()
	}

	/// Returns the `default` field of the property, if present.
	pub fn default(&self) -> Option<&Value> {
		self.value.as_object()?.get("default")
	}

	pub fn is_required(&self) -> bool {
		self.required
	}

	/// Returns the underlying JSON value of the property.
	pub fn raw_value(&self) -> &Value {
		self.value
	}
}

// endregion: --- SchemaPropRef

// region:    --- Tests

#[cfg(test)]
#[path = "schema_ref_tests.rs"]
mod tests;

// endregion: --- Tests
