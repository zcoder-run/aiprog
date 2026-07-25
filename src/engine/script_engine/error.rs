use crate::running_context::ContextRecoveryError;
use derive_more::Display;

pub type Result<T> = std::result::Result<T, crate::Error>;

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
		context: crate::RunningContext,
	},
	ContextRecovery {
		setup_source: Box<crate::Error>,
		recovery: ContextRecoveryError,
	},
}

#[derive(Debug)]
pub struct RunningEngineFinishError<T> {
	pub result: Result<T>,
	pub source: ContextRecoveryError,
}

#[derive(Debug)]
pub enum EngineExecutionError {
	Start(EngineStartError),
	Finish(RunningEngineFinishError<serde_json::Value>),
}

// region:    --- Error Boilerplate

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

impl core::fmt::Display for EngineExecutionError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Start(source) => source.fmt(f),
			Self::Finish(source) => source.fmt(f),
		}
	}
}

impl std::error::Error for EngineExecutionError {}

// endregion: --- Error Boilerplate
