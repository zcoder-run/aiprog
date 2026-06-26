type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use schemars::Schema;
use serde_json::json;

#[test]
fn test_schema_schema_ref_desc_with_description() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({
		"description": "A test schema"
	}))
	.unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let desc = schema_ref.desc();

	// -- Check
	assert_eq!(desc, Some("A test schema"), "Should return the root description");
	Ok(())
}

#[test]
fn test_schema_schema_ref_desc_without_description() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({})).unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let desc = schema_ref.desc();

	// -- Check
	assert!(desc.is_none(), "Should return None when no description");
	Ok(())
}

#[test]
fn test_schema_schema_ref_properties() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({
		"properties": {
			"name": {
				"description": "A name"
			},
			"age": {
				"default": 18
			}
		},
		"required": ["name"]
	}))
	.unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let props = schema_ref.properties();

	// -- Check
	assert_eq!(props.len(), 2, "Should have two properties");

	let name_prop = props.iter().find(|p| p.name() == "name").expect("Should have 'name' property");
	assert_eq!(name_prop.desc(), Some("A name"));
	assert!(name_prop.default().is_none());
	assert!(name_prop.is_required());

	let age_prop = props.iter().find(|p| p.name() == "age").expect("Should have 'age' property");
	assert!(age_prop.desc().is_none());
	assert_eq!(age_prop.default().and_then(|v| v.as_i64()), Some(18));
	assert!(!age_prop.is_required());

	Ok(())
}

#[test]
fn test_schema_schema_ref_properties_empty() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({"type": "object"})).unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let props = schema_ref.properties();

	// -- Check
	assert!(props.is_empty(), "Properties should be empty when none defined");

	Ok(())
}

#[test]
fn test_schema_schema_ref_raw_value() -> Result<()> {
	// -- Setup & Fixtures
	let val = json!({"description": "test", "type": "object"});
	let schema = Schema::try_from(val.clone()).unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let raw = schema_ref.raw_value();

	// -- Check
	assert_eq!(raw, &val, "raw_value should return the schema's internal value");
	Ok(())
}

#[test]
fn test_schema_schema_prop_ref_methods() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({
		"properties": {
			"count": {
				"default": 0,
				"description": "A counter"
			}
		},
		"required": ["count"]
	}))
	.unwrap();
	let schema_ref = SchemaRef::new(&schema);
	let props = schema_ref.properties();
	let prop = &props[0];

	// -- Exec & Check
	assert_eq!(prop.name(), "count");
	assert_eq!(prop.desc(), Some("A counter"));
	assert!(prop.is_required());
	let default_val = prop.default().expect("Should have a default value");
	assert_eq!(default_val.as_i64(), Some(0));
	// raw_value returns the property's JSON value
	let raw = prop.raw_value();
	assert!(raw.is_object());
	assert_eq!(raw["default"].as_i64(), Some(0));

	Ok(())
}

#[test]
fn test_schema_ref_typ_with_type() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({"type": "object"})).unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let typ = schema_ref.typ();

	// -- Check
	assert_eq!(typ, Some("object"), "Should return the type field");
	Ok(())
}

#[test]
fn test_schema_ref_typ_without_type() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({})).unwrap();
	let schema_ref = SchemaRef::new(&schema);

	// -- Exec
	let typ = schema_ref.typ();

	// -- Check
	assert!(typ.is_none(), "Should return None when no type");
	Ok(())
}

#[test]
fn test_schema_prop_ref_typ() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({
		"properties": {
			"count": {"type": "integer"}
		}
	}))
	.unwrap();
	let schema_ref = SchemaRef::new(&schema);
	let props = schema_ref.properties();
	let prop = &props[0];

	// -- Exec
	let typ = prop.typ();

	// -- Check
	assert_eq!(typ, Some("integer"), "Property should have type");
	Ok(())
}

#[test]
fn test_schema_prop_ref_typ_without_type() -> Result<()> {
	// -- Setup & Fixtures
	let schema = Schema::try_from(json!({
		"properties": {
			"count": {}
		}
	}))
	.unwrap();
	let schema_ref = SchemaRef::new(&schema);
	let props = schema_ref.properties();
	let prop = &props[0];

	// -- Exec
	let typ = prop.typ();

	// -- Check
	assert!(typ.is_none(), "Property should not have type");
	Ok(())
}
