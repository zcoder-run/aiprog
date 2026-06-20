#![allow(unused)]

// region:    --- Modules
mod registry_impl;
mod registry_internal;
mod registry_types;
pub(crate) use registry_internal::{AipHandlerClosure, RegistryEntry};
mod support;

pub use registry_impl::AipRegistry;
pub use registry_types::*;
// endregion: --- Modules

// region:    --- Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
// endregion: --- Tests
