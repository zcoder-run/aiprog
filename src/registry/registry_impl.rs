#![allow(unused)]

use crate::Result;
use crate::{AipOutput, AipParams};
use crate::{HandlerCallContext, RunningContext};
use mlua::{Lua, Value};
use schemars::schema_for;
use std::collections::HashSet;
use std::sync::Arc;

use super::HandlerError;
use super::handler_trait::AipHandler;
use super::registry_internal::{
	AipHandlerClosure, BoundRegistry, HandlerDefinition, HandlerFactory, LuaAsyncClosure, LuaSyncClosure,
};
use super::registry_types::*;
use super::support::{compile_path_patterns, validate_path};

// endregion: --- Error Boilerplate

// region:    --- Registry

#[derive(Clone)]
pub struct AipRegistry {
	inner: Arc<RegistryInner>,
}

struct RegistryInner {
	definitions: Arc<[Arc<HandlerDefinition>]>,
}

#[derive(Default)]
pub struct AipRegistryBuilder {
	definitions: Vec<Arc<HandlerDefinition>>,
	registered_paths: HashSet<String>,
}

impl AipRegistry {
	pub fn from_empty() -> AipRegistry {
		AipRegistryBuilder::default().build()
	}

	pub fn from_aip_modules() -> Result<AipRegistry> {
		crate::modules::init_registry()
	}

	pub fn to_builder(&self) -> AipRegistryBuilder {
		let definitions = self.inner.definitions.iter().cloned().collect::<Vec<_>>();
		let registered_paths = definitions
			.iter()
			.map(|definition| definition.path.clone())
			.collect();

		AipRegistryBuilder {
			definitions,
			registered_paths,
		}
	}

	pub fn list_registered_fns(&self) -> Vec<AipRegisteredFn> {
		self.inner
			.definitions
			.iter()
			.map(|definition| AipRegisteredFn {
				path: definition.path.clone(),
				params_schema: definition.params_schema.clone(),
				output_schema: definition.output_schema.clone(),
				error_schema: definition.error_schema.clone(),
				kind: definition.kind,
				description: definition.description.clone(),
				title: definition.title.clone(),
			})
			.collect()
	}

	pub fn select<I, S>(
		&self,
		patterns: I,
		options: RegistrySelectionOptions,
	) -> RegistrySelectionResult<AipRegistry>
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		transform_by_patterns(self, patterns, SelectionMode::Include, options)
	}

	pub fn exclude<I, S>(
		&self,
		patterns: I,
		options: RegistrySelectionOptions,
	) -> RegistrySelectionResult<AipRegistry>
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		transform_by_patterns(self, patterns, SelectionMode::Exclude, options)
	}

	pub(crate) async fn call(&self, lua: Lua, path: &str, value: Value) -> mlua::Result<Value> {
		let definition = self
			.inner
			.definitions
			.iter()
			.find(|e| e.path == path)
			.ok_or_else(|| mlua::Error::RuntimeError(format!("Handler not found: {path}")))?;
		let entry = definition.bind(context_free_call_context());

		match entry.handler {
			AipHandlerClosure::Sync(handler) => handler(&lua, value),
			AipHandlerClosure::Async(handler) => handler(lua, value).await,
		}
	}

	pub(crate) fn bind(&self, call_context: HandlerCallContext) -> BoundRegistry {
		BoundRegistry::from_definitions(&self.inner.definitions, call_context)
	}
}

impl AipRegistryBuilder {
	pub fn add_module<M>(self, module: M) -> Result<Self>
	where
		M: crate::AipModule,
	{
		module.register(self)
	}

	pub fn register_sync<P, R, H>(mut self, path: &str, handler: H) -> AipRegistryResult<Self>
	where
		P: AipParams,
		R: AipOutput,
		H: AipSyncFnWrapper<P, R>,
	{
		self.validate_available_path(path)?;

		let params_schema = schema_for!(P);
		let output_schema = schema_for!(R);
		let error_schema = schema_for!(HandlerError);

		let fn_path = path.to_string();
		let handler = Arc::new(handler);
		let factory: HandlerFactory = Box::new(move |call_context: HandlerCallContext| {
			let handler = Arc::clone(&handler);
			let fn_path = fn_path.clone();
			let closure: LuaSyncClosure = Box::new(move |lua: &Lua, value: Value| -> mlua::Result<Value> {
				let params: P = P::from_lua(lua, value)
					.map_err(|e| mlua::Error::RuntimeError(format!("{fn_path} - Invalid params: {e}")))?;

				match handler.call_sync(call_context.clone(), params) {
					Ok(response) => response.into_lua(lua).map_err(|e| {
						mlua::Error::RuntimeError(format!("{fn_path} - Failed to convert response to Lua: {e}"))
					}),
					Err(err) => Err(mlua::Error::RuntimeError(format!("{fn_path} - {err}"))),
				}
			});
			AipHandlerClosure::Sync(closure)
		});

		self.push_definition(HandlerDefinition {
			path: path.to_string(),
			kind: AipFnKind::Sync,
			params_schema,
			output_schema,
			error_schema,
			description: None,
			title: None,
			factory,
		});

		Ok(self)
	}

	pub fn register_async<P, O, H>(mut self, path: &str, handler: H) -> AipRegistryResult<Self>
	where
		P: AipParams,
		O: AipOutput,
		H: AipAsyncFnWrapper<P, O>,
	{
		self.validate_available_path(path)?;

		let params_schema = schema_for!(P);
		let output_schema = schema_for!(O);
		let error_schema = schema_for!(HandlerError);

		let handler = Arc::new(handler);
		let fn_path = path.to_string();
		let factory: HandlerFactory = Box::new(move |call_context: HandlerCallContext| {
			let handler = Arc::clone(&handler);
			let fn_path = fn_path.clone();
			let closure: LuaAsyncClosure = Box::new(move |lua: Lua, value: Value| {
				let handler = Arc::clone(&handler);
				let fn_path = fn_path.clone();
				let call_context = call_context.clone();

				let params = match P::from_lua(&lua, value) {
					Ok(p) => p,
					Err(e) => {
						let err_msg = format!("{fn_path} - Invalid params: {e}");
						return Box::pin(async move { Err(mlua::Error::RuntimeError(err_msg)) });
					}
				};

				Box::pin(async move {
					match handler.call_async(call_context, params).await {
						Ok(response) => response.into_lua(&lua).map_err(|e| {
							mlua::Error::RuntimeError(format!("{fn_path} - Failed to convert response to Lua: {e}"))
						}),
						Err(err) => Err(mlua::Error::RuntimeError(format!("{fn_path} - {err}"))),
					}
				})
			});
			AipHandlerClosure::Async(closure)
		});

		self.push_definition(HandlerDefinition {
			path: path.to_string(),
			kind: AipFnKind::Async,
			params_schema,
			output_schema,
			error_schema,
			description: None,
			title: None,
			factory,
		});

		Ok(self)
	}

	/// Register a handler using the [`AipHandler`] trait.
	///
	/// The handler's metadata and closure are obtained from `H::create_definition`.
	pub fn register_handler<H: AipHandler>(&mut self, path: &str, _handler: H) -> AipRegistryResult<()> {
		self.validate_available_path(path)?;
		let definition = H::create_definition(path);
		self.push_definition(definition);
		Ok(())
	}

	/// Merge all entries from `other` into `self`, consuming `other`.
	///
	/// # Errors
	/// Returns [`AipRegistryError::DuplicatePath`] if any path from `other`
	/// already exists in `self`.
	pub fn merge(mut self, other: AipRegistry) -> AipRegistryResult<Self> {
		for definition in other.inner.definitions.iter() {
			if self.registered_paths.contains(&definition.path) {
				return Err(AipRegistryError::DuplicatePath(definition.path.clone()));
			}
		}
		for definition in other.inner.definitions.iter() {
			self.registered_paths.insert(definition.path.clone());
			self.definitions.push(Arc::clone(definition));
		}
		Ok(self)
	}

	pub fn build(self) -> AipRegistry {
		AipRegistry {
			inner: Arc::new(RegistryInner {
				definitions: self.definitions.into(),
			}),
		}
	}

	fn validate_available_path(&self, path: &str) -> AipRegistryResult<()> {
		validate_path(path)?;
		if self.registered_paths.contains(path) {
			return Err(AipRegistryError::DuplicatePath(path.to_string()));
		}
		Ok(())
	}

	fn push_definition(&mut self, definition: HandlerDefinition) {
		self.registered_paths.insert(definition.path.clone());
		self.definitions.push(Arc::new(definition));
	}
}

// endregion: --- Registry

// region:    --- Support

#[derive(Clone, Copy)]
enum SelectionMode {
	Include,
	Exclude,
}

impl SelectionMode {
	fn keeps(self, matched: bool) -> bool {
		match self {
			Self::Include => matched,
			Self::Exclude => !matched,
		}
	}
}

fn transform_by_patterns<I, S>(
	source: &AipRegistry,
	patterns: I,
	mode: SelectionMode,
	options: RegistrySelectionOptions,
) -> RegistrySelectionResult<AipRegistry>
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	let matchers = compile_path_patterns(patterns)?;
	let mut matched_patterns = vec![false; matchers.len()];
	let mut definitions = Vec::new();

	for definition in source.inner.definitions.iter() {
		let mut matched = false;

		for (index, matcher) in matchers.iter().enumerate() {
			if matcher.is_match(&definition.path) {
				matched_patterns[index] = true;
				matched = true;
			}
		}

		if mode.keeps(matched) {
			definitions.push(Arc::clone(definition));
		}
	}

	if options.unmatched_patterns == UnmatchedPatternPolicy::Error
		&& let Some((matcher, _)) = matchers
			.iter()
			.zip(matched_patterns)
			.find(|(_, matched)| !matched)
	{
		return Err(RegistrySelectionError::UnmatchedPattern(
			matcher.source().to_string(),
		));
	}

	Ok(AipRegistry {
		inner: Arc::new(RegistryInner {
			definitions: definitions.into(),
		}),
	})
}

fn context_free_call_context() -> HandlerCallContext {
	let running = crate::running_context::RunningContextHandle::new(RunningContext::default());
	HandlerCallContext::new(running)
}

// endregion: --- Support
