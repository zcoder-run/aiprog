//! Home of the public engine APIs.
//!
//! Content:
//!
//! - `LuaEngine`, re-exported from the private `support` module, with its constructors,
//!   execution APIs, and documentation APIs implemented in the top-level engine files.
//! - `EngineTemplate`, `RunningEngine`, and the engine policy types.
//!
//! The internal Lua runtime wiring lives in the private `support` module, so it stays reachable
//! only from `src/engine/` and its sub-modules.

// region:    --- Modules

mod gen_doc;
mod script_engine;
mod support;

pub use script_engine::*;
pub(crate) use support::LuaEngine;

// endregion: --- Modules

// region:    --- Tests

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;

#[cfg(test)]
#[path = "engine_native_fns_tests.rs"]
mod engine_native_fns_tests;

// endregion: --- Tests
