// region:    --- Modules

mod fn_renderer;
mod gen_doc_impl;
mod type_renderer;

#[allow(unused_imports)]
pub use gen_doc_impl::generate_doc_from_fns;

// endregion: --- Modules

// region:    --- Tests

#[cfg(test)]
mod gen_doc_tests;

// endregion: --- Tests
