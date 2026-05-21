use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	#[display("Error: {_0}\n\tCause: {_1}")]
	CustomAndCause(String, String),

	// -- Externals
	#[from]
	Io(std::io::Error),

	#[from]
	Json(serde_json::Error),

	#[from]
	Lua(mlua::Error),

	#[from]
	SimpleFs(simple_fs::Error),
}

// region:    --- Custom

impl Error {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}

	/// Same as custom_and_cause (just a "cute" shorcut)
	pub fn cc(context: impl Into<String>, cause: impl std::fmt::Display) -> Self {
		Self::CustomAndCause(context.into(), cause.to_string())
	}
}

// endregion: --- Custom

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
