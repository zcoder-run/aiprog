// region:    --- Modules

mod engine_doc;
mod engine_impl;
mod engine_internal;
mod engine_native_fns;

pub use engine_impl::*;

// endregion: --- Modules

// region:    --- Tests

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;

#[cfg(test)]
#[path = "engine_native_fns_tests.rs"]
mod engine_native_fns_tests;

// endregion: --- Tests
