use crate::{AipModule, AipRegistry, AipRegistryBuilder, NativeFunctionSet};

// region:    --- Modules

mod aip_file;
mod aip_html;
mod aip_json;
mod aip_web;

pub use aip_file::file_types::{AbsolutePathPolicy, DirContext, DirPolicyError, PathPolicy, ResolvedDirPath};

// endregion: --- Modules

// region:    --- Module Types

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonModule;

#[derive(Debug, Clone, Copy, Default)]
pub struct WebModule;

#[derive(Debug, Clone, Copy, Default)]
pub struct FileModule;

#[derive(Debug, Clone, Copy, Default)]
pub struct HtmlModule;

impl AipModule for JsonModule {
	fn register(&self, builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		aip_json::register(builder)
	}
}

impl AipModule for WebModule {
	fn register(&self, builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		aip_web::register(builder)
	}
}

impl WebModule {
	#[allow(dead_code)]
	pub fn native_functions(&self) -> NativeFunctionSet {
		NativeFunctionSet::default().append_installer(aip_web::native_function_installer())
	}
}

impl AipModule for FileModule {
	fn register(&self, builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		aip_file::register::register(builder)
	}
}

impl AipModule for HtmlModule {
	fn register(&self, builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		aip_html::register(builder)
	}
}

// endregion: --- Module Types

// region:    --- Combined Registry

/// Build and return a combined `AipRegistry` containing all built-in modules
/// (`aip.json`, `aip.web`, `aip.file`).
///
/// The `aip.file` module uses a default `FileContext` (current directory).
pub fn init_registry() -> crate::Result<AipRegistry> {
	Ok(AipRegistryBuilder::default()
		.add_module(JsonModule)?
		.add_module(WebModule)?
		.add_module(FileModule)?
		.add_module(HtmlModule)?
		.build())
}

// endregion: --- Combined Registry

#[allow(dead_code)]
pub fn native_functions() -> NativeFunctionSet {
	WebModule.native_functions()
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::{
		EngineTemplate, RegistrySelectionOptions, RunningContext,
		UnmatchedPatternPolicy,
	};

	#[test]
	fn test_modules_add_module_independent_composition() -> Result<()> {
		// -- Setup & Fixtures
		let json_registry = AipRegistryBuilder::default().add_module(JsonModule)?.build();
		let web_registry = AipRegistryBuilder::default().add_module(WebModule)?.build();

		// -- Exec
		let combined = json_registry.to_builder().merge(web_registry)?.build();
		let paths = combined
			.list_registered_fns()
			.into_iter()
			.map(|registered| registered.path)
			.collect::<Vec<_>>();

		// -- Check
		assert!(paths.iter().any(|path| path == "aip.json.parse"));
		assert!(paths.iter().any(|path| path == "aip.web.get"));
		assert_eq!(json_registry.list_registered_fns().len(), 3);

		Ok(())
	}

	#[test]
	fn test_modules_select_changes_exposed_surface() -> Result<()> {
		// -- Setup & Fixtures
		let registry = init_registry()?;

		// -- Exec
		let selected = registry.select(
			["aip.json.*"],
			RegistrySelectionOptions {
				unmatched_patterns: UnmatchedPatternPolicy::Error,
			},
		)?;
		let paths = selected
			.list_registered_fns()
			.into_iter()
			.map(|registered| registered.path)
			.collect::<Vec<_>>();

		// -- Check
		assert!(!paths.is_empty());
		assert!(paths.iter().all(|path| path.starts_with("aip.json.")));

		Ok(())
	}

	#[tokio::test]
	async fn test_modules_web_native_functions_install_constants() -> Result<()> {
		// -- Setup & Fixtures
		let registry = AipRegistryBuilder::default().add_module(WebModule)?.build();
		let template = EngineTemplate::builder()
			.with_registry(registry)
			.with_native_functions(WebModule.native_functions())
			.build()?;

		// -- Exec
		let outcome = template
			.exec(
				"return { aip.web.UA_AIPROG, aip.web.UA_BROWSER }",
				RunningContext::default(),
			)
			.await?;
		let value = outcome.result?;

		// -- Check
		assert_eq!(value[0], "aiprog");
		assert!(value[1].as_str().is_some_and(|value| value.contains("Mozilla/5.0")));

		Ok(())
	}
}

// endregion: --- Tests
