#![allow(unused)]

// region:    --- Modules

mod support;

mod handler_error;
mod handler_traits;
mod handler_types;
mod registry_impl;
mod registry_internal;
mod registry_types;
mod script_error;

pub(crate) use registry_internal::{AipHandlerClosure, RegistryEntry};

pub use handler_error::*;
pub use handler_traits::*;
pub use handler_types::*;
pub use registry_impl::AipRegistry;
pub use registry_types::*;
pub use script_error::*;

// endregion: --- Modules

// region:    --- Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
// endregion: --- Tests
