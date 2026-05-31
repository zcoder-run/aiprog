use crate::script::helpers::{lua_value_to_serde_value, serde_value_to_lua_value};
use crate::script::{AipApiError, HandlerError, HandlerRegistry, RegistryError};
use mlua::{Function, Lua, MultiValue, Table, Value};
use std::sync::Arc;

// The single Lua boundary layer that bridges Lua and the `HandlerRegistry`.
//
// This is the only place that depends on `mlua`. It converts Lua params into a
// normalized `serde_json::Value`, invokes the registry, converts the normalized
// response back into a Lua value, and converts a normalized `HandlerError` into
// an `mlua::Error`. The handler, registry, and normalized boundary layers remain
// fully Lua-agnostic.

// region:    --- Value Conversion

/// Convert a single Lua params argument into a normalized `serde_json::Value`.
///
/// The empty-table case (`aip.fn({})`) and a missing argument (`aip.fn()`) both
/// become an empty normalized object, matching the empty-object special case
/// handled in `handler_params.rs`.
pub fn lua_params_to_value(lua_value: Value) -> mlua::Result<serde_json::Value> {
	let value = match lua_value {
		// A missing argument is treated as an empty params object.
		Value::Nil => serde_json::Value::Object(serde_json::Map::new()),
		other => lua_value_to_serde_value(other)
			.map_err(|err| mlua::Error::RuntimeError(format!("Failed to convert Lua params to JSON: {err}")))?,
	};
	Ok(value)
}

/// Convert a normalized `serde_json::Value` response back into a Lua value,
/// preserving the root-level `data` response shape.
pub fn value_to_lua_response(lua: &Lua, value: serde_json::Value) -> mlua::Result<Value> {
	serde_value_to_lua_value(lua, value)
		.map_err(|err| mlua::Error::RuntimeError(format!("Failed to convert response to Lua: {err}")))
}

// endregion: --- Value Conversion

// region:    --- Error Conversion

/// Convert a normalized `HandlerError` into an `mlua::Error`.
///
/// When the handler error carries a typed `AipApiError`, the error code,
/// message, and optional details/cause are surfaced. A `RegistryError` is
/// surfaced with its display message. Otherwise, the error type name is used as
/// a fallback.
pub fn handler_error_to_lua(mut err: HandlerError) -> mlua::Error {
	if let Some(api_err) = err.remove::<AipApiError>() {
		return api_error_to_lua(api_err);
	}

	if let Some(reg_err) = err.get::<RegistryError>() {
		return mlua::Error::RuntimeError(reg_err.to_string());
	}

	mlua::Error::RuntimeError(format!("Handler error: {}", err.type_name()))
}

fn api_error_to_lua(api_err: AipApiError) -> mlua::Error {
	let mut msg = format!("[{}] {}", api_err.code, api_err.message);
	if let Some(details) = api_err.details.as_ref() {
		msg.push_str(&format!("\nDetails: {details}"));
	}
	if let Some(cause) = api_err.cause.as_ref() {
		msg.push_str(&format!("\nCause: {cause}"));
	}
	mlua::Error::RuntimeError(msg)
}

// endregion: --- Error Conversion

// region:    --- Registration

/// Install all functions of a `HandlerRegistry` onto the given Lua module table.
///
/// The registry is shared across all installed functions via an `Arc`. Each
/// installed Lua function:
/// - converts its single params argument into a normalized `serde_json::Value`,
/// - invokes the registry handler by name,
/// - converts the normalized response back into a Lua value,
/// - converts any normalized handler error into an `mlua::Error`.
///
/// This replaces the per-function `mlua` glue with a single shared adapter.
pub fn install_registry_on_table(lua: &Lua, table: &Table, registry: HandlerRegistry) -> mlua::Result<()> {
	let registry = Arc::new(registry);

	let names: Vec<String> = registry.list_registered_fns().into_iter().map(|reg_fn| reg_fn.name).collect();

	for name in names {
		let func = make_registry_function(lua, registry.clone(), name.clone())?;
		table.set(name, func)?;
	}

	Ok(())
}

/// Build a single Lua async function bound to a registry entry by `name`.
fn make_registry_function(lua: &Lua, registry: Arc<HandlerRegistry>, name: String) -> mlua::Result<Function> {
	lua.create_async_function(move |lua: Lua, args: MultiValue| {
		let registry = registry.clone();
		let name = name.clone();
		async move {
			let arg = args.into_iter().next().unwrap_or(Value::Nil);
			let params_value = lua_params_to_value(arg)?;

			let response_value = registry.call(&name, params_value).await.map_err(handler_error_to_lua)?;

			let response_lua = value_to_lua_response(&lua, response_value)?;
			Ok::<Value, mlua::Error>(response_lua)
		}
	})
}

// endregion: --- Registration

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::script::{AipApiError, HandlerRegistry};
	use mlua::Lua;
	use serde::{Deserialize, Serialize};

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[derive(Debug, Deserialize, schemars::JsonSchema)]
	struct EchoParams {
		#[serde(default)]
		data: String,
	}

	#[derive(Debug, Serialize, schemars::JsonSchema)]
	struct EchoResult {
		data: String,
	}

	fn echo_sync(params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Ok(EchoResult { data: params.data })
	}

	fn fail_sync(_params: EchoParams) -> core::result::Result<EchoResult, AipApiError> {
		Err(AipApiError::new("PARSE_FAILED", "boom").with_details("some details"))
	}

	#[tokio::test]
	async fn test_lua_adapter_install_and_call_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = Lua::new();
		let module = lua.create_table()?;
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;
		install_registry_on_table(&lua, &module, registry)?;
		lua.globals().set("m", module)?;

		// -- Exec
		let res: String = lua
			.load(r#"local r = m.echo({ data = "hello" }); return r.data"#)
			.eval_async()
			.await?;

		// -- Check
		assert_eq!(res, "hello");

		Ok(())
	}

	#[tokio::test]
	async fn test_lua_adapter_empty_table_ok() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = Lua::new();
		let module = lua.create_table()?;
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("echo", echo_sync)?;
		install_registry_on_table(&lua, &module, registry)?;
		lua.globals().set("m", module)?;

		// -- Exec
		let res: String = lua.load(r#"local r = m.echo({}); return r.data"#).eval_async().await?;

		// -- Check
		assert_eq!(res, "");

		Ok(())
	}

	#[tokio::test]
	async fn test_lua_adapter_error_thrown() -> TestResult<()> {
		// -- Setup & Fixtures
		let lua = Lua::new();
		let module = lua.create_table()?;
		let mut registry = HandlerRegistry::new();
		registry.append_sync::<_, EchoParams, EchoResult, AipApiError>("fail", fail_sync)?;
		install_registry_on_table(&lua, &module, registry)?;
		lua.globals().set("m", module)?;

		// -- Exec
		let ok: bool = lua
			.load(r#"local ok, err = pcall(m.fail, { data = "x" }); return ok"#)
			.eval_async()
			.await?;

		// -- Check
		assert!(!ok);

		Ok(())
	}
}

// endregion: --- Tests
