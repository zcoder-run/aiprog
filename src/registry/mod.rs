#![allow(unused)]

// region:    --- Modules

mod support;

mod handler_error;
mod handler_trait;
mod handler_traits;
mod module_trait;
pub mod handler_types;
mod registry_impl;
pub mod registry_internal;
mod registry_types;

pub(crate) use registry_internal::AipHandlerClosure;
pub use registry_internal::HandlerDefinition;

pub use handler_error::*;
pub use handler_trait::AipHandler;
pub use handler_traits::*;
pub use handler_types::*;
pub use module_trait::AipModule;
pub use registry_impl::{AipRegistry, AipRegistryBuilder};
pub use registry_types::*;

// endregion: --- Modules

// region:    --- Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
// endregion: --- Tests
