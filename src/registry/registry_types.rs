use crate::{AipOutput, AipParams, HandlerCallContext};
use schemars::Schema;
use std::future::Future;
use std::pin::Pin;

use super::handler_error::HandlerResult;

// region:    --- Handler Wrappers

pub type AipAsyncBoxFuture<R> = Pin<Box<dyn Future<Output = HandlerResult<R>> + Send>>;

pub trait AipSyncFnWrapper<P, R>: Send + Sync + 'static
where
	P: AipParams,
	R: AipOutput,
{
	fn call_sync(&self, call_context: HandlerCallContext, params: P) -> HandlerResult<R>;
}

impl<H, P, R> AipSyncFnWrapper<P, R> for H
where
	H: Fn(HandlerCallContext, P) -> HandlerResult<R> + Send + Sync + 'static,
	P: AipParams,
	R: AipOutput,
{
	fn call_sync(&self, call_context: HandlerCallContext, params: P) -> HandlerResult<R> {
		self(call_context, params)
	}
}

pub trait AipAsyncFnWrapper<P, R>: Send + Sync + 'static
where
	P: AipParams,
	R: AipOutput,
{
	fn call_async(&self, call_context: HandlerCallContext, params: P) -> AipAsyncBoxFuture<R>;
}

impl<H, Fut, P, R> AipAsyncFnWrapper<P, R> for H
where
	H: Fn(HandlerCallContext, P) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = HandlerResult<R>> + Send + 'static,
	P: AipParams,
	R: AipOutput,
{
	fn call_async(&self, call_context: HandlerCallContext, params: P) -> AipAsyncBoxFuture<R> {
		Box::pin(self(call_context, params))
	}
}

// endregion: --- Handler Wrappers

// region:    --- Metadata Types

#[derive(Debug, Clone)]
pub struct AipRegisteredFn {
	pub path: String,
	pub params_schema: Schema,
	pub output_schema: Schema,
	pub error_schema: Schema,
	pub kind: AipFnKind,
	pub description: Option<String>,
	pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AipFnKind {
	Sync,
	Async,
}

// endregion: --- Metadata Types

// region:    --- Registry Error

pub type AipRegistryResult<T> = core::result::Result<T, AipRegistryError>;

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

// region:    --- Registry Selection

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnmatchedPatternPolicy {
	#[default]
	Allow,
	Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegistrySelectionOptions {
	pub unmatched_patterns: UnmatchedPatternPolicy,
}

pub type RegistrySelectionResult<T> = core::result::Result<T, RegistrySelectionError>;

#[derive(Debug, Clone, derive_more::Display)]
pub enum RegistrySelectionError {
	#[display("Invalid registry selection pattern '{pattern}': {reason}")]
	InvalidPattern { pattern: String, reason: String },

	#[display("Registry selection pattern matched no paths: {_0}")]
	UnmatchedPattern(String),
}

impl std::error::Error for RegistrySelectionError {}

// endregion: --- Registry Selection
