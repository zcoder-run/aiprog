mod web_tester;

pub use web_tester::*;

pub type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.
