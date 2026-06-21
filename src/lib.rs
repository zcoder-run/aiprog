// region:    --- Modules
extern crate self as aiprog; // for aiprog_macros

#[cfg(test)]
mod _test_support;

mod script;
mod support;

mod error;

pub use error::{Error, Result};

// NOTE: for now, re-export one by one to tune the shape of this crate.
pub use script::AipRegistry;
pub use script::{AipApiError, AipApiResult, AipError, AipFnKind, AipOutput, AipParams, AipRegistryError};
pub use script::{AipFromLua, AipIntoLua, LuaExt, LuaJsonExt};
pub use script::{ScriptEngine, ScriptError, ScriptResult};

pub mod types;
pub mod webc;

// -- Re-exports for macro support
pub use mlua;
pub use serde_json;

pub mod macros {
	pub use aiprog_macros::*;
}

// endregion: --- Modules
