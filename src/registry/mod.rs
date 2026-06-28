#![allow(unused)]

// region:    --- Modules

mod support;

mod handler_error;
mod handler_traits;
pub(crate) mod handler_types;
mod handler_trait;
mod registry_impl;
pub(crate) mod registry_internal;
mod registry_types;

pub(crate) use registry_internal::{AipHandlerClosure, RegistryEntry};

pub use handler_error::*;
pub use handler_traits::*;
pub use handler_types::*;
pub use handler_trait::AipHandler;
pub use registry_impl::AipRegistry;
pub use registry_types::*;

// endregion: --- Modules

// region:    --- Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
// endregion: --- Tests
