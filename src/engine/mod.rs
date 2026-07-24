// region:    --- Modules

mod engine_doc;
mod engine_impl;
mod engine_internal;
mod engine_native_fns;
mod engine_template;

pub use engine_impl::*;
pub use engine_template::*;

// endregion: --- Modules

// region:    --- Tests

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;

#[cfg(test)]
#[path = "engine_native_fns_tests.rs"]
mod engine_native_fns_tests;

// endregion: --- Tests
