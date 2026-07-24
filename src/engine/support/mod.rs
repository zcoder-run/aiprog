//! Private engine support modules.
//!
//! These modules own the internal Lua runtime wiring used by the engine. They are declared
//! under a private `support` module, so only `src/engine/` and its sub-modules can use them,
//! while the public engine API remains exposed from the top-level `src/engine/` files.
//!
//! Module boundaries:
//!
//! - `lua_engine`, the concrete internal `LuaEngine` state and engine-only support APIs.
//! - `native_fns`, initialization of the native Lua helper functions.
//! - `lua_path`, shared dotted Lua path installation for values and functions.

// region:    --- Modules

mod lua_engine;
mod lua_engine_register;
mod lua_path;
mod native_fns;

pub use lua_engine::*;
pub use lua_path::*;

// endregion: --- Modules
