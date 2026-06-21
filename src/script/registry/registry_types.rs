use crate::script::{AipError, AipParams, AipOutput};
use schemars::Schema;
use std::future::Future;
use std::pin::Pin;

// region:    --- Handler Wrappers

pub type AipAsyncBoxFuture<R, E> = Pin<Box<dyn Future<Output = core::result::Result<R, E>> + Send>>;

pub trait AipSyncFnWrapper<P, R, E>: Send + Sync + 'static
where
	P: AipParams,
	R: AipOutput,
	E: AipError,
{
	fn call_sync(&self, params: P) -> core::result::Result<R, E>;
}

impl<H, P, R, E> AipSyncFnWrapper<P, R, E> for H
where
	H: Fn(P) -> core::result::Result<R, E> + Send + Sync + 'static,
	P: AipParams,
	R: AipOutput,
	E: AipError,

{
	fn call_sync(&self, params: P) -> core::result::Result<R, E> {
		self(params)
	}
}

pub trait AipAsyncFnWrapper<P, R, E>: Send + Sync + 'static
where
	P: AipParams,
	R: AipOutput,
	E: AipError,
{
	fn call_async(&self, params: P) -> AipAsyncBoxFuture<R, E>;
}

impl<H, Fut, P, R, E> AipAsyncFnWrapper<P, R, E> for H
where
	H: Fn(P) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = core::result::Result<R, E>> + Send + 'static,
	P: AipParams,
	R: AipOutput,
	E: AipError,

{
	fn call_async(&self, params: P) -> AipAsyncBoxFuture<R, E> {
		Box::pin(self(params))
	}
}

// endregion: --- Handler Wrappers

// region:    --- Metadata Types

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
