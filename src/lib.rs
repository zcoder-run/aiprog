// region:    --- Modules

#[cfg(test)]
mod _test_support;

mod support;

mod error;
mod script;

pub use error::{Error, Result};
pub use script::{ScriptEngine, ScriptError, ScriptResult};

pub mod registry;
pub mod types;
pub mod webc;

// endregion: --- Modules
