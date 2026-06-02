// region:    --- Modules

#[cfg(test)]
mod _test_support;

mod support;

mod error;
mod script;

pub use error::{Error, Result};
pub use script::ScriptEngine;

pub mod registry;
pub mod types;

// endregion: --- Modules
