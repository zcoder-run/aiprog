use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

/// Error for the webc module.
#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
	#[from(String, &String, &str)]
	#[display("Error: {_0}")]
	Custom(String),

	#[display("Build failed: {_0}")]
	BuildFailed(String),

	#[display("Request failed: {_0}")]
	RequestFailed(String),

	#[display("Body parse failed: {_0}")]
	BodyParseFailed(String),
}

// region:    --- Custom

impl Error {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn build_failed(val: impl Into<String>) -> Self {
		Self::BuildFailed(val.into())
	}

	pub fn request_failed(val: impl Into<String>) -> Self {
		Self::RequestFailed(val.into())
	}

	pub fn body_parse_failed(val: impl Into<String>) -> Self {
		Self::BodyParseFailed(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}
}

// endregion: --- Custom

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
