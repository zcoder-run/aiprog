// region:    --- Modules
#![doc = include_str!("../docs/rustdoc/lib.md")]

extern crate self as aiprog; // for aiprog_macros

#[cfg(test)]
mod _test_support;

mod modules;
pub mod schema_ref;
mod support;

mod engine;
mod error;
mod error_lua_details;
mod lua_exts;
pub mod registry;
mod run_outcome;
pub(crate) mod running_context;

pub use error::{Error, Result};
pub use error_lua_details::LuaErrorDetails;
pub use modules::{AbsolutePathPolicy, DirContext, DirPolicyError, PathPolicy, ResolvedDirPath};
pub use run_outcome::RunOutcome;
pub use running_context::{ContextAccessError, ContextRecoveryError, HandlerCallContext, RunningContext};

// NOTE: for now, re-export one by one to tune the shape of this crate.
pub use engine::*;
pub use lua_exts::*;
pub use registry::*;

pub mod types;
pub mod webc;

// -- Re-exports for macro support
pub use mlua;
pub use serde_json;

pub mod derive {
	pub use aiprog_macros::{AipError, AipFromLua, AipIntoLua, AipOutput, AipParams};
}

// endregion: --- Modules

// -- Re-export handler attribute macro
pub use aiprog_macros::aip_handler;
pub use aiprog_macros::register_handler;
