// region:    --- Tests

use super::*;
use crate::{AipIntoLua, HandlerCallContext, impl_lua_serde_traits};
use mlua::Lua;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct TestParams {
	data: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct TestResponse {
	data: String,
}

impl_lua_serde_traits!(TestParams);
impl_lua_serde_traits!(TestResponse);

impl crate::AipParams for TestParams {}
impl crate::AipOutput for TestResponse {}

fn test_sync_handler(_call: HandlerCallContext, params: TestParams) -> HandlerResult<TestResponse> {
	Ok(TestResponse { data: params.data })
}

async fn test_async_handler(_call: HandlerCallContext, params: TestParams) -> HandlerResult<TestResponse> {
	Ok(TestResponse { data: params.data })
}

// region:    --- Sync registration

#[test]
fn test_registry_register_sync_success() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default();

	// -- Exec
	let registry = registry.register_sync("test.parse", test_sync_handler)?.build();
	let fns = registry.list_registered_fns();

	// -- Check
	assert_eq!(fns.len(), 1);
	assert_eq!(fns[0].path, "test.parse");
	assert_eq!(fns[0].kind, AipFnKind::Sync);

	Ok(())
}

// endregion: --- Sync registration

// region:    --- Async registration

#[test]
fn test_registry_register_async_success() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default();

	// -- Exec
	let registry = registry.register_async("test.fetch", test_async_handler)?.build();
	let fns = registry.list_registered_fns();

	// -- Check
	assert_eq!(fns.len(), 1);
	assert_eq!(fns[0].path, "test.fetch");
	assert_eq!(fns[0].kind, AipFnKind::Async);

	Ok(())
}

// endregion: --- Async registration

// region:    --- Duplicate path

#[test]
fn test_registry_duplicate_path_rejection() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default().register_sync("test.parse", test_sync_handler)?;

	// -- Exec
	let result = registry.register_sync("test.parse", test_sync_handler);

	// -- Check
	assert!(result.is_err(), "expected duplicate path error");
	let err = result
		.as_ref()
		.err()
		.ok_or("expected duplicate path error")?;
	assert!(matches!(err, AipRegistryError::DuplicatePath(_)));

	Ok(())
}

// endregion: --- Duplicate path

// region:    --- Invalid paths

#[test]
fn test_registry_invalid_paths() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default();

	// -- Exec & Check
	assert!(AipRegistryBuilder::default().register_sync("", test_sync_handler).is_err());
	assert!(AipRegistryBuilder::default().register_sync("parse", test_sync_handler).is_err());
	assert!(AipRegistryBuilder::default().register_sync("test.", test_sync_handler).is_err());
	assert!(AipRegistryBuilder::default().register_sync(".parse", test_sync_handler).is_err());
	assert!(
		AipRegistryBuilder::default()
			.register_sync("test..parse", test_sync_handler)
			.is_err()
	);

	Ok(())
}

// endregion: --- Invalid paths

// region:    --- Schema metadata

#[test]
fn test_registry_schema_metadata() -> Result<()> {
	// -- Setup & Fixtures
	let registry = AipRegistryBuilder::default();

	// -- Exec
	let registry = registry.register_sync("test.parse", test_sync_handler)?.build();
	let fns = registry.list_registered_fns();
	let registered = &fns[0];
	let params_schema = serde_json::to_value(&registered.params_schema)?;
	let output_schema = serde_json::to_value(&registered.output_schema)?;
	let error_schema = serde_json::to_value(&registered.error_schema)?;

	// -- Check
	assert!(!params_schema.is_null());
	assert!(!output_schema.is_null());
	assert!(!error_schema.is_null());

	Ok(())
}

// endregion: --- Schema metadata

// region:    --- Merge

#[test]
fn test_registry_merge_success() -> Result<()> {
	let reg1 = AipRegistryBuilder::default().register_sync("test.parse", test_sync_handler)?;

	let reg2 = AipRegistryBuilder::default()
		.register_async("test.fetch", test_async_handler)?
		.build();

	// merge
	let reg1 = reg1.merge(reg2)?.build();

	let fns = reg1.list_registered_fns();
	assert_eq!(fns.len(), 2);
	assert!(fns.iter().any(|f| f.path == "test.parse"));
	assert!(fns.iter().any(|f| f.path == "test.fetch"));

	Ok(())
}

#[test]
fn test_registry_merge_duplicate() -> Result<()> {
	let reg1 = AipRegistryBuilder::default().register_sync("test.parse", test_sync_handler)?;

	let reg2 = AipRegistryBuilder::default()
		.register_sync("test.parse", test_sync_handler)?
		.build();

	let result = reg1.merge(reg2);
	assert!(result.is_err());
	assert!(matches!(result, Err(AipRegistryError::DuplicatePath(_))));

	Ok(())
}

// endregion: --- Merge

// region:    --- Handler context

#[tokio::test]
async fn test_registry_handler_context_access() -> Result<()> {
	// -- Setup & Fixtures
	fn context_handler(call: HandlerCallContext, params: TestParams) -> HandlerResult<TestResponse> {
		let prefix = call
			.with::<String, _>(Clone::clone)
			.map_err(HandlerError::custom_from_err)?;
		Ok(TestResponse {
			data: format!("{prefix}{}", params.data),
		})
	}

	let registry = AipRegistryBuilder::default();

	// -- Exec
	let registry = registry.register_sync("test.context", context_handler)?.build();
	let lua = Lua::new();
	let params = TestParams {
		data: String::from("value"),
	};
	let value = params.into_lua(&lua)?;
	let result = registry.call(lua, "test.context", value).await;

	// -- Check
	assert!(result.is_err(), "expected missing context error");

	Ok(())
}

// endregion: --- Handler context

// endregion: --- Tests

#[test]
fn test_registry_to_builder_preserves_source_and_order() -> Result<()> {
	// -- Setup & Fixtures
	let source = AipRegistryBuilder::default()
		.register_sync("test.parse", test_sync_handler)?
		.build();

	// -- Exec
	let extended = source
		.to_builder()
		.register_async("test.fetch", test_async_handler)?
		.build();
	let source_fns = source.list_registered_fns();
	let extended_fns = extended.list_registered_fns();

	// -- Check
	assert_eq!(source_fns.len(), 1);
	assert_eq!(extended_fns.len(), 2);
	assert_eq!(extended_fns[0].path, "test.parse");
	assert_eq!(extended_fns[1].path, "test.fetch");

	Ok(())
}

// region:    --- Selection

#[test]
fn test_registry_select_patterns_matching_and_order() -> Result<()> {
	// -- Setup & Fixtures
	let source = AipRegistryBuilder::default()
		.register_sync("aip.json.parse", test_sync_handler)?
		.register_sync("aip.json.stringify", test_sync_handler)?
		.register_sync("aip.web", test_sync_handler)?
		.register_sync("aip.web.get", test_sync_handler)?
		.register_sync("aip.web.auth.login", test_sync_handler)?
		.register_sync("app.lookup", test_sync_handler)?
		.build();

	// -- Exec
	let selected = source.select(
		["aip.json.*", "aip.web.**", "aip.json.parse"],
		RegistrySelectionOptions::default(),
	)?;
	let literal = source.select(["app.lookup"], RegistrySelectionOptions::default())?;
	let selected_paths = selected
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();
	let literal_paths = literal
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();

	// -- Check
	assert_eq!(
		selected_paths,
		[
			"aip.json.parse",
			"aip.json.stringify",
			"aip.web",
			"aip.web.get",
			"aip.web.auth.login",
		]
	);
	assert_eq!(literal_paths, ["app.lookup"]);

	Ok(())
}

#[test]
fn test_registry_exclude_patterns_matching_and_order() -> Result<()> {
	// -- Setup & Fixtures
	let source = AipRegistryBuilder::default()
		.register_sync("aip.json.parse", test_sync_handler)?
		.register_sync("aip.web.get", test_sync_handler)?
		.register_sync("aip.web.auth.login", test_sync_handler)?
		.register_sync("app.lookup", test_sync_handler)?
		.build();

	// -- Exec
	let excluded = source.exclude(
		["aip.web.**", "app.lookup"],
		RegistrySelectionOptions::default(),
	)?;
	let paths = excluded
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();

	// -- Check
	assert_eq!(paths, ["aip.json.parse"]);

	Ok(())
}

#[test]
fn test_registry_select_invalid_and_unmatched_patterns() -> Result<()> {
	// -- Setup & Fixtures
	let source = AipRegistryBuilder::default()
		.register_sync("aip.json.parse", test_sync_handler)?
		.build();
	let strict_options = RegistrySelectionOptions {
		unmatched_patterns: UnmatchedPatternPolicy::Error,
	};

	// -- Exec
	let empty_segment = source.select(["aip..*"], RegistrySelectionOptions::default());
	let embedded_wildcard = source.select(["aip.j*"], RegistrySelectionOptions::default());
	let unmatched = source.select(["aip.web.*"], strict_options);
	let allowed_unmatched = source.select(["aip.web.*"], RegistrySelectionOptions::default())?;

	// -- Check
	assert!(matches!(
		empty_segment,
		Err(RegistrySelectionError::InvalidPattern { .. })
	));
	assert!(matches!(
		embedded_wildcard,
		Err(RegistrySelectionError::InvalidPattern { .. })
	));
	assert!(matches!(
		unmatched,
		Err(RegistrySelectionError::UnmatchedPattern(_))
	));
	assert!(allowed_unmatched.list_registered_fns().is_empty());

	Ok(())
}

#[test]
fn test_registry_select_extension_preserves_source_independence() -> Result<()> {
	// -- Setup & Fixtures
	let source = AipRegistryBuilder::default()
		.register_sync("aip.json.parse", test_sync_handler)?
		.register_sync("aip.web.get", test_sync_handler)?
		.build();
	let selected = source.select(["aip.json.*"], RegistrySelectionOptions::default())?;
	let additional = AipRegistryBuilder::default()
		.register_sync("audit.record", test_sync_handler)?
		.build();

	// -- Exec
	let extended = selected
		.to_builder()
		.register_sync("app.lookup", test_sync_handler)?
		.merge(additional)?
		.build();
	let source_paths = source
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();
	let selected_paths = selected
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();
	let extended_paths = extended
		.list_registered_fns()
		.into_iter()
		.map(|function| function.path)
		.collect::<Vec<_>>();

	// -- Check
	assert_eq!(source_paths, ["aip.json.parse", "aip.web.get"]);
	assert_eq!(selected_paths, ["aip.json.parse"]);
	assert_eq!(
		extended_paths,
		["aip.json.parse", "app.lookup", "audit.record"]
	);

	Ok(())
}

// endregion: --- Selection
