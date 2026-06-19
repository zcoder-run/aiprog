// region:    --- Modules
extern crate self as aiprog;

#[cfg(test)]
mod _test_support;

mod support;

mod error;
pub mod script;

pub use error::{Error, Result};
pub use script::{ScriptEngine, ScriptError, ScriptResult};

pub mod registry;
pub mod types;
pub mod webc;

// -- Re-exports for macro support
pub use aiprog_macros::{AipError, AipFromLua, AipIntoLua, AipParams, AipResponse};
pub use mlua;
pub use serde_json;

// endregion: --- Modules
