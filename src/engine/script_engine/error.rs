use crate::running_context::ContextRecoveryError;
use derive_more::Display;
use derive_more::From;

pub(crate) type EngineResult<T> = std::result::Result<T, EngineError>;

#[derive(Debug, From)]
pub enum EngineError {
	#[from]
	Build(EngineBuildError),
	#[from]
	Start(EngineStartError),
	#[from]
	FinishRecovery(Box<RunningEngineFinishError<serde_json::Value>>),
	#[from(String, &str)]
	Custom(String),
}

// region:    --- Froms

impl From<mlua::Error> for EngineError {
	fn from(err: mlua::Error) -> Self {
		EngineError::Custom(err.to_string())
	}
}

impl From<RunningEngineFinishError<serde_json::Value>> for EngineError {
	fn from(err: RunningEngineFinishError<serde_json::Value>) -> Self {
		EngineError::FinishRecovery(Box::new(err))
	}
}

// endregion: --- Froms

// region:    --- Types

#[derive(Debug, Display)]
pub enum EngineBuildError {
	#[display("An engine requires a registry")]
	MissingRegistry,

	#[display("The base Lua standard library cannot be disabled")]
	BaseLibraryRequired,

	#[display("Lua instruction limits are not supported by the current runtime")]
	InstructionLimitUnsupported,

	#[display("Lua wall-clock limits are not supported by the current runtime")]
	WallClockTimeoutUnsupported,
}

#[derive(Debug)]
pub enum EngineStartError {
	Setup {
		source: Box<crate::Error>,
		#[allow(dead_code)]
		context: crate::RunningContext,
	},
	ContextRecovery {
		setup_source: Box<crate::Error>,
		recovery: ContextRecoveryError,
	},
}

#[derive(Debug)]
pub struct RunningEngineFinishError<T> {
	#[allow(dead_code)]
	pub result: EngineResult<T>,
	pub source: ContextRecoveryError,
}

// endregion: --- Types

// region:    --- Error Boilerplate

impl core::fmt::Display for EngineError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Build(e) => write!(f, "{e}"),
			Self::Start(e) => write!(f, "{e}"),
			Self::FinishRecovery(e) => write!(f, "{e}"),
			Self::Custom(s) => write!(f, "{s}"),
		}
	}
}

impl std::error::Error for EngineBuildError {}

impl core::fmt::Display for EngineStartError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Setup { source, .. } => write!(f, "Failed to start isolated engine: {source}"),
			Self::ContextRecovery { setup_source, recovery } => write!(
				f,
				"Failed to start isolated engine ({setup_source}) and recover its context: {recovery}"
			),
		}
	}
}

impl std::error::Error for EngineStartError {}

impl<T> core::fmt::Display for RunningEngineFinishError<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Failed to recover running context: {}", self.source)
	}
}

impl<T: core::fmt::Debug> std::error::Error for RunningEngineFinishError<T> {}

impl std::error::Error for EngineError {}

// endregion: --- Error Boilerplate
