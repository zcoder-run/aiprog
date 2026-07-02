#![allow(unused)]

// region:    --- Modules

mod support;

mod handler_error;
mod handler_trait;
mod handler_traits;
pub mod handler_types;
mod registry_impl;
pub mod registry_internal;
mod registry_types;

pub(crate) use registry_internal::AipHandlerClosure;
pub use registry_internal::RegistryEntry;

pub use handler_error::*;
pub use handler_trait::AipHandler;
pub use handler_traits::*;
pub use handler_types::*;
pub use registry_impl::AipRegistry;
pub use registry_types::*;

// endregion: --- Modules

// region:    --- Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
// endregion: --- Tests
