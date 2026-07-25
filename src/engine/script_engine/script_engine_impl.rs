use super::error::{EngineBuildError, EngineError, EngineStartError, Result, RunningEngineFinishError};
use super::lua_runtime_policy::LuaRuntimePolicy;
use crate::engine::LuaEngine;
use crate::running_context::RunningContextHandle;
use crate::{AipRegistry, HandlerCallContext, RunOutcome, RunningContext};
use mlua::{Lua, LuaOptions, StdLib};
use std::sync::Arc;

pub type NativeFunctionInstaller = Arc<dyn Fn(&Lua) -> mlua::Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct NativeFunctionSet {
	installers: Arc<[NativeFunctionInstaller]>,
}

#[derive(Clone)]
pub struct ScriptEngine {
	inner: Arc<ScriptEngineInner>,
}

struct ScriptEngineInner {
	registry: AipRegistry,
	lua_policy: LuaRuntimePolicy,
	native_functions: NativeFunctionSet,
}

#[derive(Default)]
pub struct ScriptEngineBuilder {
	registry: Option<AipRegistry>,
	lua_policy: LuaRuntimePolicy,
	native_functions: NativeFunctionSet,
}

pub struct RunningEngine {
	engine: LuaEngine,
	context: RunningContextHandle,
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

impl ScriptEngine {
	pub fn builder() -> ScriptEngineBuilder {
		ScriptEngineBuilder::default()
	}

	pub fn generate_doc(&self) -> Result<String> {
		let engine = LuaEngine::from_context_free_registry(self.inner.registry.clone())
			.map_err(|e| EngineError::Custom(e.to_string()))?;
		engine.generate_doc().map_err(|e| EngineError::Custom(e.to_string()))
	}

	pub fn start(&self, context: RunningContext) -> core::result::Result<RunningEngine, EngineStartError> {
		let context_handle = RunningContextHandle::new(context);
		let call_context = HandlerCallContext::new(context_handle.clone());

		match self.create_running_engine(call_context, context_handle.clone()) {
			Ok(engine) => Ok(engine),
			Err(setup_source) => match context_handle.recover() {
				Ok(context) => Err(EngineStartError::Setup {
					source: Box::new(crate::Error::Engine(setup_source)),
					context,
				}),
				Err(recovery) => Err(EngineStartError::ContextRecovery {
					setup_source: Box::new(crate::Error::Engine(setup_source)),
					recovery,
				}),
			},
		}
	}

	pub async fn exec(&self, script: &str, context: RunningContext) -> Result<RunOutcome<serde_json::Value>> {
		let running = self.start(context)?;
		let outcome = running.exec(script).await?;
		Ok(outcome)
	}

	fn create_running_engine(
		&self,
		call_context: HandlerCallContext,
		context: RunningContextHandle,
	) -> Result<RunningEngine> {
		let lua = create_restricted_lua(&self.inner.lua_policy)?;
		let mut engine = LuaEngine {
			lua,
			registered_fns: Vec::new(),
		};
		engine.init_native_fns().map_err(|e| EngineError::Custom(e.to_string()))?;
		engine
			.register_with_context(self.inner.registry.clone(), call_context)
			.map_err(|e| EngineError::Custom(e.to_string()))?;
		self.inner.native_functions.install(engine.lua())?;

		Ok(RunningEngine { engine, context })
	}
}

impl ScriptEngineBuilder {
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
	pub fn build(self) -> Result<ScriptEngine> {
		validate_runtime_policy(&self.lua_policy)?;
		let registry = self.registry.ok_or(EngineBuildError::MissingRegistry)?;

		Ok(ScriptEngine {
			inner: Arc::new(ScriptEngineInner {
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
		let engine_result = result.map_err(|e| EngineError::Custom(e.to_string()));
		drop(engine);

		let context = match context.recover() {
			Ok(context) => context,
			Err(source) => {
				return Err(RunningEngineFinishError {
					result: engine_result,
					source,
				});
			}
		};

		let crate_result = engine_result.map_err(crate::Error::Engine);
		Ok(RunOutcome::new(crate_result, context))
	}
}

// region:    --- Support
fn validate_runtime_policy(policy: &LuaRuntimePolicy) -> Result<()> {
	if !policy.std_lib_policy().base {
		return Err(EngineBuildError::BaseLibraryRequired.into());
	}
	if policy.limits().max_instructions.is_some() {
		return Err(EngineBuildError::InstructionLimitUnsupported.into());
	}
	if policy.limits().wall_clock_timeout.is_some() {
		return Err(EngineBuildError::WallClockTimeoutUnsupported.into());
	}
	Ok(())
}

fn create_restricted_lua(policy: &LuaRuntimePolicy) -> mlua::Result<Lua> {
	let std_lib_policy = &policy.std_lib_policy();
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
	if let Some(max_memory_bytes) = policy.limits().max_memory_bytes {
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
	async fn test_script_engine_exec_uses_fresh_lua_state() -> Result<()> {
		// -- Setup & Fixtures
		let engine = ScriptEngine::builder().with_registry(AipRegistry::from_empty()).build()?;

		// -- Exec
		let first = engine.exec("leaked_value = 42; return true", RunningContext::default()).await?;
		let second = engine.exec("return leaked_value == nil", RunningContext::default()).await?;

		// -- Check
		assert_eq!(first.result?, serde_json::Value::Bool(true));
		assert_eq!(second.result?, serde_json::Value::Bool(true));

		Ok(())
	}

	#[tokio::test]
	async fn test_script_engine_exec_returns_context_after_script_error() -> Result<()> {
		// -- Setup & Fixtures
		let engine = ScriptEngine::builder().with_registry(AipRegistry::from_empty()).build()?;
		let mut context = RunningContext::default();
		context.insert::<u32>(42);

		// -- Exec
		let outcome = engine.exec("this is not valid Lua", context).await?;

		// -- Check
		assert!(outcome.result.is_err());
		assert_eq!(outcome.context.get::<u32>(), Some(&42));

		Ok(())
	}

	#[test]
	fn test_script_engine_start_returns_context_after_setup_error() -> Result<()> {
		// -- Setup & Fixtures
		let installer: NativeFunctionInstaller =
			Arc::new(|_| Err(mlua::Error::RuntimeError("Forced native installer failure".into())));
		let native_functions = NativeFunctionSet::default().append_installer(installer);
		let engine = ScriptEngine::builder()
			.with_registry(AipRegistry::from_empty())
			.with_native_functions(native_functions)
			.build()?;
		let mut context = RunningContext::default();
		context.insert::<u32>(42);

		// -- Exec
		let error = engine.start(context).err().ok_or("Should return a setup error")?;

		// -- Check
		let EngineStartError::Setup { source, context } = error else {
			return Err("Expected setup error with recovered context".into());
		};
		assert!(source.to_string().contains("Forced native installer failure"));
		assert_eq!(context.get::<u32>(), Some(&42));

		Ok(())
	}

	#[test]
	fn test_script_engine_build_rejects_missing_registry() -> Result<()> {
		// -- Setup & Fixtures
		let builder = ScriptEngine::builder();

		// -- Exec
		let error = builder.build().err().ok_or("Should reject a gengine without a registry")?;

		// -- Check
		assert!(matches!(error, EngineError::Build(EngineBuildError::MissingRegistry)));

		Ok(())
	}
}

// endregion: --- Tests
