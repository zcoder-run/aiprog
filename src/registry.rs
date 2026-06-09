#![allow(unused)]

use crate::script::{AipError, AipParams, AipResponse, IntoHandlerError, handler_error_to_lua};
use mlua::{Lua, Value};
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// region:    --- Handler Wrappers

pub type AipAsyncBoxFuture<R, E> = Pin<Box<dyn Future<Output = core::result::Result<R, E>> + Send>>;

pub trait AipSyncFnWrapper<P, R, E>: Send + Sync + 'static
where
	P: AipParams,
	R: AipResponse,
	E: AipError,
{
	fn call_sync(&self, params: P) -> core::result::Result<R, E>;
}

impl<H, P, R, E> AipSyncFnWrapper<P, R, E> for H
where
	H: Fn(P) -> core::result::Result<R, E> + Send + Sync + 'static,
	P: AipParams,
	R: AipResponse,
	E: AipError,
{
	fn call_sync(&self, params: P) -> core::result::Result<R, E> {
		self(params)
	}
}

pub trait AipAsyncFnWrapper<P, R, E>: Send + Sync + 'static
where
	P: AipParams,
	R: AipResponse,
	E: AipError,
{
	fn call_async(&self, params: P) -> AipAsyncBoxFuture<R, E>;
}

impl<H, Fut, P, R, E> AipAsyncFnWrapper<P, R, E> for H
where
	H: Fn(P) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = core::result::Result<R, E>> + Send + 'static,
	P: AipParams,
	R: AipResponse,
	E: AipError,
{
	fn call_async(&self, params: P) -> AipAsyncBoxFuture<R, E> {
		Box::pin(self(params))
	}
}

// endregion: --- Handler Wrappers

// region:    --- Closure Type Aliases

pub(crate) type LuaSyncClosure = Box<dyn Fn(&Lua, Value) -> mlua::Result<Value> + Send + Sync>;

pub(crate) type LuaAsyncClosure =
	Box<dyn Fn(&Lua, Value) -> Pin<Box<dyn Future<Output = mlua::Result<serde_json::Value>> + Send>> + Send + Sync>;

// endregion: --- Closure Type Aliases

// region:    --- Metadata Types

use schemars::Schema;

#[derive(Debug, Clone)]
pub struct AipRegisteredFn {
	pub path: String,
	pub params_schema: Schema,
	pub response_schema: Schema,
	pub error_schema: Schema,
	pub kind: AipFnKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AipFnKind {
	Sync,
	Async,
}

// endregion: --- Metadata Types

// region:    --- Registry Error

#[derive(Debug, Clone, derive_more::Display)]
pub enum AipRegistryError {
	// -- Path validation
	#[display("Invalid path: {_0}")]
	InvalidPath(String),

	// -- Duplicate registration
	#[display("Path already registered: {_0}")]
	DuplicatePath(String),

	// -- Schema generation
	#[display("Failed to generate schema: {_0}")]
	SchemaError(String),

	// -- Type-erased handler setup
	#[display("Handler setup error: {_0}")]
	HandlerSetup(String),
}

// endregion: --- Registry Error

// region:    --- Error Boilerplate

impl std::error::Error for AipRegistryError {}

// endregion: --- Error Boilerplate

// region:    --- Registry

use std::collections::HashSet;

#[derive(Default)]
pub struct AipRegistry {
	pub(crate) entries: Vec<RegistryEntry>,
	registered_paths: HashSet<String>,
}

pub(crate) struct RegistryEntry {
	pub path: String,
	pub kind: AipFnKind,
	pub handler: AipHandlerClosure,
	pub params_schema: Schema,
	pub response_schema: Schema,
	pub error_schema: Schema,
}

pub(crate) enum AipHandlerClosure {
	Sync(LuaSyncClosure),
	Async(LuaAsyncClosure),
}

impl AipRegistry {
	pub fn register_sync<P, R, E, H>(&mut self, path: &str, handler: H) -> core::result::Result<(), AipRegistryError>
	where
		P: AipParams,
		R: AipResponse,
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
				Err(err) => Err(handler_error_to_lua(err.into_handler_error())),
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

	pub fn register_async<P, R, E, H>(&mut self, path: &str, handler: H) -> core::result::Result<(), AipRegistryError>
	where
		P: AipParams,
		R: AipResponse + serde::Serialize,
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
						Err(err) => Err(handler_error_to_lua(err.into_handler_error())),
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
}

fn validate_path(path: &str) -> core::result::Result<(), AipRegistryError> {
	if path.is_empty() {
		return Err(AipRegistryError::InvalidPath("Path must not be empty".into()));
	}
	let segments: Vec<&str> = path.split('.').collect();
	if segments.len() < 2 {
		return Err(AipRegistryError::InvalidPath(format!(
			"Path '{}' must have at least one module/namespace segment and a function name segment",
			path
		)));
	}
	for seg in &segments {
		if seg.is_empty() {
			return Err(AipRegistryError::InvalidPath(format!(
				"Path '{}' contains empty segment(s)",
				path
			)));
		}
	}
	Ok(())
}

// endregion: --- Registry

// region:    --- Tests

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;

// endregion: --- Tests
