#![allow(unused)]

use crate::LuaJsonExt;
use crate::Result;
use crate::{AipError, AipOutput, AipParams};
use mlua::{Lua, Value};
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::registry_internal::{AipHandlerClosure, LuaAsyncClosure, LuaSyncClosure, RegistryEntry};
use super::registry_types::*;
use super::support::validate_path;

// endregion: --- Error Boilerplate

// region:    --- Registry

use std::collections::HashSet;

pub struct AipRegistry {
	pub(crate) entries: Vec<RegistryEntry>,
	registered_paths: HashSet<String>,
}

impl AipRegistry {
	pub fn from_empty() -> AipRegistry {
		AipRegistry {
			entries: Default::default(),
			registered_paths: Default::default(),
		}
	}

	pub fn from_aip_modules() -> Result<AipRegistry> {
		crate::modules::init_registry()
	}
}

impl AipRegistry {
	pub fn register_sync<P, R, E, H>(&mut self, path: &str, handler: H) -> AipRegistryResult<()>
	where
		P: AipParams,
		R: AipOutput,
		E: AipError,
		H: AipSyncFnWrapper<P, R, E>,
	{
		validate_path(path)?;
		if self.registered_paths.contains(path) {
			return Err(AipRegistryError::DuplicatePath(path.to_string()));
		}

		let params_schema = schema_for!(P);
		let response_schema = schema_for!(R);
		let error_schema = schema_for!(E);

		let closure: LuaSyncClosure = Box::new(move |lua: &Lua, value: Value| -> mlua::Result<Value> {
			let params: P =
				P::from_lua(lua, value).map_err(|e| mlua::Error::RuntimeError(format!("Invalid params: {e}")))?;

			match handler.call_sync(params) {
				Ok(response) => response
					.into_lua(lua)
					.map_err(|e| mlua::Error::RuntimeError(format!("Failed to convert response to Lua: {e}"))),
                Err(err) => Err(err.into().into_lua_error()),
			}
		});

		self.registered_paths.insert(path.to_string());
		self.entries.push(RegistryEntry {
			path: path.to_string(),
			kind: AipFnKind::Sync,
			handler: AipHandlerClosure::Sync(closure),
			params_schema,
			response_schema,
			error_schema,
		});

		Ok(())
	}

	pub fn register_async<P, R, E, H>(&mut self, path: &str, handler: H) -> AipRegistryResult<()>
	where
		P: AipParams,
		R: AipOutput + serde::Serialize,
		E: AipError,
		H: AipAsyncFnWrapper<P, R, E>,
	{
		validate_path(path)?;
		if self.registered_paths.contains(path) {
			return Err(AipRegistryError::DuplicatePath(path.to_string()));
		}

		let params_schema = schema_for!(P);
		let response_schema = schema_for!(R);
		let error_schema = schema_for!(E);

		let handler_arc = Arc::new(handler);
		let closure: LuaAsyncClosure = Box::new(
			move |lua: &Lua, value: Value| -> Pin<Box<dyn Future<Output = mlua::Result<serde_json::Value>> + Send>> {
				let handler = Arc::clone(&handler_arc);

				let params = match P::from_lua(lua, value) {
					Ok(p) => p,
					Err(e) => {
						let err_msg = format!("Invalid params: {e}");
						return Box::pin(async move { Err(mlua::Error::RuntimeError(err_msg)) });
					}
				};

				Box::pin(async move {
					match handler.call_async(params).await {
						Ok(response) => serde_json::to_value(response)
							.map_err(|e| mlua::Error::RuntimeError(format!("Failed to serialize async response: {e}"))),
                        Err(err) => Err(err.into().into_lua_error()),
					}
				})
			},
		);

		self.registered_paths.insert(path.to_string());
		self.entries.push(RegistryEntry {
			path: path.to_string(),
			kind: AipFnKind::Async,
			handler: AipHandlerClosure::Async(closure),
			params_schema,
			response_schema,
			error_schema,
		});

		Ok(())
	}

	/// Merge all entries from `other` into `self`, consuming `other`.
	///
	/// # Errors
	/// Returns [`AipRegistryError::DuplicatePath`] if any path from `other`
	/// already exists in `self`.
	pub fn merge(&mut self, other: AipRegistry) -> AipRegistryResult<()> {
		for entry in &other.entries {
			if self.registered_paths.contains(&entry.path) {
				return Err(AipRegistryError::DuplicatePath(entry.path.clone()));
			}
		}
		self.registered_paths.extend(other.registered_paths);
		self.entries.extend(other.entries);
		Ok(())
	}

	pub fn list_registered_fns(&self) -> Vec<AipRegisteredFn> {
		self.entries
			.iter()
			.map(|entry| AipRegisteredFn {
				path: entry.path.clone(),
				params_schema: entry.params_schema.clone(),
				response_schema: entry.response_schema.clone(),
				error_schema: entry.error_schema.clone(),
				kind: entry.kind,
			})
			.collect()
	}

	pub(crate) async fn call(&self, lua: Lua, path: &str, value: Value) -> mlua::Result<Value> {
		let entry = self
			.entries
			.iter()
			.find(|e| e.path == path)
			.ok_or_else(|| mlua::Error::RuntimeError(format!("Handler not found: {path}")))?;

		match &entry.handler {
			AipHandlerClosure::Sync(handler) => handler(&lua, value),
			AipHandlerClosure::Async(handler) => {
				let json_value = handler(&lua, value).await?;
				// Convert serde_json::Value back to Lua Value.
				Value::x_from_json_value(&lua, json_value)
					.map_err(|e| mlua::Error::RuntimeError(format!("Failed to convert JSON to Lua: {e}")))
			}
		}
	}
}

// endregion: --- Registry
