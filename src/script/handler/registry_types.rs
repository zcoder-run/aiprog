use super::{AipError, AipParams, AipResponse};
use crate::script::HandlerResult;
use crate::script::{HandlerError, HandlerWrapperTrait, IntoHandlerWrapper};
use mlua::{Lua, Value};
use schemars::{Schema, schema_for};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

// region:    --- Registry Error

/// Errors that can occur while building or invoking the registry.
///
/// The registry stays fully Lua-agnostic; conversion of these errors into
/// `mlua::Error` is the responsibility of the Lua adapter layer.
#[derive(Debug, Clone, Serialize, derive_more::Display)]
pub enum RegistryError {
	// -- Path validation
	#[display("Invalid path: {_0}")]
	InvalidPath(String),

	// -- Duplicate registration
	#[display("Path already registered: {_0}")]
	DuplicatePath(String),

	// -- Method lookup
	#[display("Method unknown: {_0}")]
	MethodUnknown(String),
}

impl std::error::Error for RegistryError {}

// endregion: --- Registry Error

// region:    --- Metadata Types

/// Whether a registered handler is synchronous or asynchronous.
///
/// This is informational metadata captured at registration time. The call
/// path is uniform (async future) regardless of kind, since the handler trait
/// always returns a future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HandlerKind {
	Sync,
	Async,
}

/// Public, cloneable metadata describing a registered function, suitable for
/// documentation and tooling.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RegisteredFn {
	pub name: String,
	pub kind: HandlerKind,
	pub params_schema: Schema,
	pub response_schema: Schema,
	pub error_schema: Schema,
}

// endregion: --- Metadata Types

// region:    --- Registry Entry

/// Internal per-function registry entry, holding the boxed type-erased handler
/// wrapper along with its schema metadata.
#[allow(dead_code)]
struct RegistryEntry {
	name: &'static str,
	kind: HandlerKind,
	wrapper: Box<dyn HandlerWrapperTrait>,
	params_schema: Schema,
	response_schema: Schema,
	error_schema: Schema,
}

// endregion: --- Registry Entry

// region:    --- Registry

/// A Lua-agnostic registry that stores concrete handler metadata and
/// type-erased handler wrappers.
///
/// Modeled on `rpc-router::RouterInner`, but adapted to the single
/// params-table shape used by `AipFn`. It exposes a builder-style append API
/// and a call API that invokes a handler by name with `mlua::Value` params.
/// The registry never observes `mlua` types directly (the wrapper trait does).
#[derive(Default)]
#[allow(dead_code)]
pub struct HandlerRegistry {
	entries: HashMap<&'static str, RegistryEntry>,
}

impl fmt::Debug for HandlerRegistry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("HandlerRegistry")
			.field("entries", &self.entries.keys())
			.finish()
	}
}
#[allow(dead_code)]
impl HandlerRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	/// Append a typed sync handler under `name`, capturing its schema metadata.
	///
	/// The handler is boxed into a type-erased `HandlerWrapperTrait` so it can
	/// be stored and invoked dynamically by the registry.
	pub fn append_sync<H, P, R, E>(&mut self, name: &'static str, handler: H) -> core::result::Result<(), RegistryError>
	where
		H: crate::script::Handler<P, R, crate::script::SyncMarker> + Clone + Send + Sync + 'static,
		P: AipParams + crate::script::AipFromLua,
		R: AipResponse + crate::script::AipIntoLua,
		E: AipError,
	{
		self.insert_entry::<P, R, E>(name, HandlerKind::Sync, handler.into_dyn())
	}

	/// Append a typed async handler under `name`, capturing its schema metadata.
	///
	/// The handler is boxed into a type-erased `HandlerWrapperTrait` so it can
	/// be stored and invoked dynamically by the registry.
	pub fn append_async<H, P, R, E>(
		&mut self,
		name: &'static str,
		handler: H,
	) -> core::result::Result<(), RegistryError>
	where
		H: crate::script::Handler<P, R, crate::script::AsyncMarker> + Clone + Send + Sync + 'static,
		P: AipParams + crate::script::AipFromLua,
		R: AipResponse + crate::script::AipIntoLua,
		E: AipError,
	{
		self.insert_entry::<P, R, E>(name, HandlerKind::Async, handler.into_dyn())
	}

	/// Invoke a registered handler by `name` with a Lua state and a Lua value
	/// params argument, returning a Lua value response or a normalized `HandlerError`.
	///
	/// An unknown method yields a `HandlerError` carrying a `RegistryError`.
	pub async fn call(&self, lua: Lua, name: &str, params_value: Value) -> HandlerResult<Value> {
		match self.entries.get(name) {
			Some(entry) => entry.wrapper.call(&lua, params_value).await,
			None => Err(HandlerError::Registry(RegistryError::MethodUnknown(name.to_string()))),
		}
	}

	/// List public, cloneable metadata for all registered functions.
	pub fn list_registered_fns(&self) -> Vec<RegisteredFn> {
		self.entries
			.values()
			.map(|entry| RegisteredFn {
				name: entry.name.to_string(),
				kind: entry.kind,
				params_schema: entry.params_schema.clone(),
				response_schema: entry.response_schema.clone(),
				error_schema: entry.error_schema.clone(),
			})
			.collect()
	}

	fn insert_entry<P, R, E>(
		&mut self,
		name: &'static str,
		kind: HandlerKind,
		wrapper: Box<dyn HandlerWrapperTrait>,
	) -> core::result::Result<(), RegistryError>
	where
		P: AipParams,
		R: AipResponse,
		E: AipError,
	{
		validate_name(name)?;
		if self.entries.contains_key(name) {
			return Err(RegistryError::DuplicatePath(name.to_string()));
		}

		let entry = RegistryEntry {
			name,
			kind,
			wrapper,
			params_schema: schema_for!(P),
			response_schema: schema_for!(R),
			error_schema: schema_for!(E),
		};

		self.entries.insert(name, entry);
		Ok(())
	}
}

#[allow(dead_code)]
fn validate_name(name: &str) -> core::result::Result<(), RegistryError> {
	if name.is_empty() {
		return Err(RegistryError::InvalidPath("Name must not be empty".into()));
	}
	Ok(())
}

// endregion: --- Registry

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::impl_lua_serde_traits;
	use crate::script::AipApiError;
	use crate::script::LuaJsonExt;
	use serde::{Deserialize, Serialize};
	use serde_json::json;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
	struct EchoParams {
		data: String,
	}

	#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
	struct EchoResult {
		data: String,
	}

	impl_lua_serde_traits!(EchoParams);
	impl_lua_serde_traits!(EchoResult);

	impl crate::script::AipParams for EchoParams {}
	impl crate::script::AipResponse for EchoResult {}

	fn echo_sync(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	async fn echo_async(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	#[tokio::test]
	async fn test_registry_sync_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = mlua::Lua::new();
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;
		let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": "hello" }))
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let value = registry.call(lua.clone(), "echo", params_lua).await?;

		// -- Check
		let back_json = value.x_to_json_value().map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		assert_eq!(back_json, Some(json!({ "data": "hello" })));

		Ok(())
	}

	#[tokio::test]
	async fn test_registry_async_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = mlua::Lua::new();
		let mut registry = HandlerRegistry::new();
		registry.append_async::<_, EchoParams, EchoResult, AipApiError>("echo", echo_async)?;
		let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": "world" }))
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let value = registry.call(lua.clone(), "echo", params_lua).await?;

		// -- Check
		let back_json = value
			.x_to_json_value()
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?
			.ok_or_else(|| mlua::Error::RuntimeError("expected JSON value".to_string()))?;
		assert_eq!(back_json, json!({ "data": "world" }));

		Ok(())
	}

	#[tokio::test]
	async fn test_registry_method_unknown_err() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = mlua::Lua::new();
		let registry = HandlerRegistry::new();
		let params_lua = mlua::Value::Nil;

		// -- Exec
		let res = registry.call(lua, "missing", params_lua).await;

		// -- Check
		let err = res.err().ok_or("should be an error")?;
		assert!(
			matches!(err, HandlerError::Registry(RegistryError::MethodUnknown(_))),
			"Expected Registry error, got {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn test_registry_duplicate_path_err() -> TestResult<()> {
		// -- Setup & Fixtures
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;

		// -- Exec
		let res = registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync);

		// -- Check
		let err = res.err().ok_or("should be an error")?;
		assert!(matches!(err, RegistryError::DuplicatePath(_)));

		Ok(())
	}

	#[tokio::test]
	async fn test_registry_invalid_params_err() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = mlua::Lua::new();
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;
		let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "data": 123 }))
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		// -- Exec
		let res = registry.call(lua.clone(), "echo", params_lua).await;

		// -- Check
		let err = res.err().ok_or("should be an error")?;
		// After simplifying to an enum, invalid params produce a HandlerError::Script variant.
		assert!(
			matches!(err, HandlerError::Script(_)),
			"Expected Script error, got {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn test_registry_list_registered_fns_contains_schemas() -> TestResult<()> {
		// -- Setup & Fixtures
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;

		// -- Exec
		let fns = registry.list_registered_fns();

		// -- Check
		assert_eq!(fns.len(), 1);
		let registered = &fns[0];
		assert_eq!(registered.name, "echo");
		assert!(matches!(registered.kind, HandlerKind::Sync));
		// Schemas should resolve to something non‑null
		assert!(!registered.params_schema.clone().to_value().is_null());
		assert!(!registered.response_schema.clone().to_value().is_null());
		assert!(!registered.error_schema.clone().to_value().is_null());
		Ok(())
	}
}

// endregion: --- Tests
