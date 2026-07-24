use crate::{
	AipRegistry, ContextRecoveryError, Error, HandlerCallContext, Result, RunOutcome, RunningContext,
	ScriptEngine,
};
use derive_more::Display;
use mlua::{Lua, LuaOptions, StdLib};
use std::sync::Arc;
use std::time::Duration;

pub type NativeFunctionInstaller = Arc<dyn Fn(&Lua) -> mlua::Result<()> + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct LuaRuntimePolicy {
	std_lib_policy: LuaStdLibPolicy,
	limits: LuaExecutionLimits,
}

#[derive(Debug, Clone)]
pub struct LuaStdLibPolicy {
	base: bool,
	coroutine: bool,
	math: bool,
	string: bool,
	table: bool,
	utf8: bool,
	package: bool,
	io: bool,
	os: bool,
	debug: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LuaExecutionLimits {
	max_memory_bytes: Option<usize>,
	max_instructions: Option<u64>,
	wall_clock_timeout: Option<Duration>,
}

#[derive(Clone)]
pub struct NativeFunctionSet {
	installers: Arc<[NativeFunctionInstaller]>,
}

#[derive(Clone)]
pub struct EngineTemplate {
	inner: Arc<EngineTemplateInner>,
}

struct EngineTemplateInner {
	registry: AipRegistry,
	lua_policy: LuaRuntimePolicy,
	native_functions: NativeFunctionSet,
}

#[derive(Default)]
pub struct EngineTemplateBuilder {
	registry: Option<AipRegistry>,
	lua_policy: LuaRuntimePolicy,
	native_functions: NativeFunctionSet,
}

pub struct RunningEngine {
	engine: ScriptEngine,
	context: crate::running_context::RunningContextHandle,
}

#[derive(Debug, Display)]
pub enum TemplateBuildError {
	#[display("An engine template requires a registry")]
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
		source: Box<Error>,
		context: RunningContext,
	},
	ContextRecovery {
		setup_source: Box<Error>,
		recovery: ContextRecoveryError,
	},
}

#[derive(Debug)]
pub struct RunningEngineFinishError<T> {
	pub result: Result<T>,
	pub source: ContextRecoveryError,
}

#[derive(Debug)]
pub enum TemplateExecutionError {
	Start(EngineStartError),
	Finish(RunningEngineFinishError<serde_json::Value>),
}

impl LuaRuntimePolicy {
	pub fn with_std_lib_policy(mut self, policy: LuaStdLibPolicy) -> Self {
		self.std_lib_policy = policy;
		self
	}

	pub fn with_limits(mut self, limits: LuaExecutionLimits) -> Self {
		self.limits = limits;
		self
	}

	pub fn std_lib_policy(&self) -> &LuaStdLibPolicy {
		&self.std_lib_policy
	}

	pub fn limits(&self) -> &LuaExecutionLimits {
		&self.limits
	}
}

impl Default for LuaStdLibPolicy {
	fn default() -> Self {
		Self {
			base: true,
			coroutine: true,
			math: true,
			string: true,
			table: true,
			utf8: true,
			package: false,
			io: false,
			os: false,
			debug: false,
		}
	}
}

impl LuaStdLibPolicy {
	pub fn with_base(mut self, enabled: bool) -> Self {
		self.base = enabled;
		self
	}

	pub fn with_coroutine(mut self, enabled: bool) -> Self {
		self.coroutine = enabled;
		self
	}

	pub fn with_math(mut self, enabled: bool) -> Self {
		self.math = enabled;
		self
	}

	pub fn with_string(mut self, enabled: bool) -> Self {
		self.string = enabled;
		self
	}

	pub fn with_table(mut self, enabled: bool) -> Self {
		self.table = enabled;
		self
	}

	pub fn with_utf8(mut self, enabled: bool) -> Self {
		self.utf8 = enabled;
		self
	}

	pub fn with_package(mut self, enabled: bool) -> Self {
		self.package = enabled;
		self
	}

	pub fn with_io(mut self, enabled: bool) -> Self {
		self.io = enabled;
		self
	}

	pub fn with_os(mut self, enabled: bool) -> Self {
		self.os = enabled;
		self
	}

	pub fn with_debug(mut self, enabled: bool) -> Self {
		self.debug = enabled;
		self
	}
}

impl LuaExecutionLimits {
	pub fn with_max_memory_bytes(mut self, max_memory_bytes: usize) -> Self {
		self.max_memory_bytes = Some(max_memory_bytes);
		self
	}

	pub fn with_max_instructions(mut self, max_instructions: u64) -> Self {
		self.max_instructions = Some(max_instructions);
		self
	}

	pub fn with_wall_clock_timeout(mut self, wall_clock_timeout: Duration) -> Self {
		self.wall_clock_timeout = Some(wall_clock_timeout);
		self
	}

	pub fn max_memory_bytes(&self) -> Option<usize> {
		self.max_memory_bytes
	}

	pub fn max_instructions(&self) -> Option<u64> {
		self.max_instructions
	}

	pub fn wall_clock_timeout(&self) -> Option<Duration> {
		self.wall_clock_timeout
	}
}

impl NativeFunctionSet {
	pub fn new(installers: impl Into<Arc<[NativeFunctionInstaller]>>) -> Self {
		Self {
			installers: installers.into(),
		}
	}

	pub fn append_installer(mut self, installer: NativeFunctionInstaller) -> Self {
		let mut installers = self.installers.iter().cloned().collect::<Vec<_>>();
		installers.push(installer);
		self.installers = installers.into();
		self
	}

	fn install(&self, lua: &Lua) -> mlua::Result<()> {
		for installer in self.installers.iter() {
			installer(lua)?;
		}
		Ok(())
	}
}

impl Default for NativeFunctionSet {
	fn default() -> Self {
		Self {
			installers: Arc::from([]),
		}
	}
}

impl EngineTemplate {
	pub fn builder() -> EngineTemplateBuilder {
		EngineTemplateBuilder::default()
	}

	pub fn start(&self, context: RunningContext) -> core::result::Result<RunningEngine, EngineStartError> {
		let context_handle = crate::running_context::RunningContextHandle::new(context);
		let call_context = HandlerCallContext::new(context_handle.clone());

		match self.create_running_engine(call_context, context_handle.clone()) {
			Ok(engine) => Ok(engine),
			Err(setup_source) => match context_handle.recover() {
				Ok(context) => Err(EngineStartError::Setup {
					source: Box::new(setup_source),
					context,
				}),
				Err(recovery) => Err(EngineStartError::ContextRecovery {
					setup_source: Box::new(setup_source),
					recovery,
				}),
			},
		}
	}

	pub async fn exec(
		&self,
		script: &str,
		context: RunningContext,
	) -> core::result::Result<RunOutcome<serde_json::Value>, TemplateExecutionError> {
		let running = self.start(context).map_err(TemplateExecutionError::Start)?;
		running.exec(script).await.map_err(TemplateExecutionError::Finish)
	}

	fn create_running_engine(
		&self,
		call_context: HandlerCallContext,
		context: crate::running_context::RunningContextHandle,
	) -> Result<RunningEngine> {
		let lua = create_restricted_lua(&self.inner.lua_policy)?;
		let mut engine = ScriptEngine {
			lua,
			registered_fns: Vec::new(),
		};
		engine.init_native_fns()?;
		engine.register_with_context(self.inner.registry.clone(), call_context)?;
		self.inner.native_functions.install(engine.lua())?;

		Ok(RunningEngine { engine, context })
	}
}

impl EngineTemplateBuilder {
	pub fn with_registry(mut self, registry: AipRegistry) -> Self {
		self.registry = Some(registry);
		self
	}

	pub fn with_lua_policy(mut self, policy: LuaRuntimePolicy) -> Self {
		self.lua_policy = policy;
		self
	}

	pub fn with_native_functions(mut self, native_functions: NativeFunctionSet) -> Self {
		self.native_functions = native_functions;
		self
	}

	pub fn build(self) -> core::result::Result<EngineTemplate, TemplateBuildError> {
		validate_runtime_policy(&self.lua_policy)?;
		let registry = self.registry.ok_or(TemplateBuildError::MissingRegistry)?;

		Ok(EngineTemplate {
			inner: Arc::new(EngineTemplateInner {
				registry,
				lua_policy: self.lua_policy,
				native_functions: self.native_functions,
			}),
		})
	}
}

impl RunningEngine {
	pub async fn exec(
		self,
		script: &str,
	) -> core::result::Result<RunOutcome<serde_json::Value>, RunningEngineFinishError<serde_json::Value>> {
		let Self { engine, context } = self;
		let result = engine.exec(script).await;
		drop(engine);

		let context = match context.recover() {
			Ok(context) => context,
			Err(source) => return Err(RunningEngineFinishError { result, source }),
		};

		Ok(RunOutcome::new(result, context))
	}
}

// region:    --- Error Boilerplate

impl std::error::Error for TemplateBuildError {}

impl core::fmt::Display for EngineStartError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Setup { source, .. } => write!(f, "Failed to start isolated engine: {source}"),
			Self::ContextRecovery {
				setup_source,
				recovery,
			} => write!(
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

impl core::fmt::Display for TemplateExecutionError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Start(source) => source.fmt(f),
			Self::Finish(source) => source.fmt(f),
		}
	}
}

impl std::error::Error for TemplateExecutionError {}

// endregion: --- Error Boilerplate

// region:    --- Support

fn validate_runtime_policy(
	policy: &LuaRuntimePolicy,
) -> core::result::Result<(), TemplateBuildError> {
	if !policy.std_lib_policy.base {
		return Err(TemplateBuildError::BaseLibraryRequired);
	}
	if policy.limits.max_instructions.is_some() {
		return Err(TemplateBuildError::InstructionLimitUnsupported);
	}
	if policy.limits.wall_clock_timeout.is_some() {
		return Err(TemplateBuildError::WallClockTimeoutUnsupported);
	}
	Ok(())
}

fn create_restricted_lua(policy: &LuaRuntimePolicy) -> mlua::Result<Lua> {
	let std_lib_policy = &policy.std_lib_policy;
	let mut std_libs = StdLib::NONE;

	if std_lib_policy.coroutine {
		std_libs |= StdLib::COROUTINE;
	}
	if std_lib_policy.math {
		std_libs |= StdLib::MATH;
	}
	if std_lib_policy.string {
		std_libs |= StdLib::STRING;
	}
	if std_lib_policy.table {
		std_libs |= StdLib::TABLE;
	}
	if std_lib_policy.utf8 {
		std_libs |= StdLib::UTF8;
	}
	if std_lib_policy.package {
		std_libs |= StdLib::PACKAGE;
	}
	if std_lib_policy.io {
		std_libs |= StdLib::IO;
	}
	if std_lib_policy.os {
		std_libs |= StdLib::OS;
	}
	if std_lib_policy.debug {
		std_libs |= StdLib::DEBUG;
	}

	let lua = Lua::new_with(std_libs, LuaOptions::default())?;
	if let Some(max_memory_bytes) = policy.limits.max_memory_bytes {
		lua.set_memory_limit(max_memory_bytes)?;
	}

	Ok(lua)
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_engine_engine_template_exec_uses_fresh_lua_state() -> Result<()> {
		// -- Setup & Fixtures
		let template = EngineTemplate::builder()
			.with_registry(AipRegistry::from_empty())
			.build()?;

		// -- Exec
		let first = template
			.exec("leaked_value = 42; return true", RunningContext::default())
			.await?;
		let second = template
			.exec("return leaked_value == nil", RunningContext::default())
			.await?;

		// -- Check
		assert_eq!(first.result?, serde_json::Value::Bool(true));
		assert_eq!(second.result?, serde_json::Value::Bool(true));

		Ok(())
	}

	#[tokio::test]
	async fn test_engine_engine_template_exec_returns_context_after_script_error() -> Result<()> {
		// -- Setup & Fixtures
		let template = EngineTemplate::builder()
			.with_registry(AipRegistry::from_empty())
			.build()?;
		let mut context = RunningContext::default();
		context.insert::<u32>(42);

		// -- Exec
		let outcome = template.exec("this is not valid Lua", context).await?;

		// -- Check
		assert!(outcome.result.is_err());
		assert_eq!(outcome.context.get::<u32>(), Some(&42));

		Ok(())
	}

	#[test]
	fn test_engine_engine_template_start_returns_context_after_setup_error() -> Result<()> {
		// -- Setup & Fixtures
		let installer: NativeFunctionInstaller = Arc::new(|_| {
			Err(mlua::Error::RuntimeError(
				"Forced native installer failure".into(),
			))
		});
		let native_functions = NativeFunctionSet::default().append_installer(installer);
		let template = EngineTemplate::builder()
			.with_registry(AipRegistry::from_empty())
			.with_native_functions(native_functions)
			.build()?;
		let mut context = RunningContext::default();
		context.insert::<u32>(42);

		// -- Exec
		let error = template
			.start(context)
			.err()
			.ok_or("Should return a setup error")?;

		// -- Check
		let EngineStartError::Setup { source, context } = error else {
			return Err("Expected setup error with recovered context".into());
		};
		assert!(source.to_string().contains("Forced native installer failure"));
		assert_eq!(context.get::<u32>(), Some(&42));

		Ok(())
	}

	#[test]
	fn test_engine_engine_template_build_rejects_missing_registry() -> Result<()> {
		// -- Setup & Fixtures
		let builder = EngineTemplate::builder();

		// -- Exec
		let error = builder
			.build()
			.err()
			.ok_or("Should reject a template without a registry")?;

		// -- Check
		assert!(matches!(error, TemplateBuildError::MissingRegistry));

		Ok(())
	}
}

// endregion: --- Tests
