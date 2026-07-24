use super::LuaEngine;
use crate::AipHandlerClosure;
use crate::running_context::RunningContextHandle;
use crate::{AipFnKind, AipRegistry};
use crate::{HandlerCallContext, Result, RunningContext};
use mlua::{Lua, MultiValue, Value};

use super::install_function_at_path;

impl LuaEngine {
	pub(in crate::engine) fn register(&mut self, registry: AipRegistry) -> Result<()> {
		self.register_with_context(registry, context_free_call_context())
	}

	pub(in crate::engine) fn register_with_context(
		&mut self,
		registry: AipRegistry,
		call_context: HandlerCallContext,
	) -> Result<()> {
		let lua = &self.lua;
		self.registered_fns = registry.list_registered_fns();

		for entry in registry.bind(call_context) {
			let func = match entry.kind {
				AipFnKind::Sync => {
					let handler = if let AipHandlerClosure::Sync(handler) = entry.handler {
						handler
					} else {
						return Err("Mismatched handler kind for sync entry".into());
					};
					lua.create_function(move |lua: &Lua, args: MultiValue| -> mlua::Result<Value> {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						handler(lua, arg)
					})?
				}

				AipFnKind::Async => {
					let handler = if let AipHandlerClosure::Async(handler) = entry.handler {
						handler
					} else {
						return Err("Mismatched handler kind for async entry".into());
					};

					lua.create_async_function(move |lua: Lua, args: MultiValue| {
						let arg = args.into_iter().next().unwrap_or(Value::Nil);
						let response_fut = handler(lua, arg);
						async move {
							let response_lua = response_fut.await?;
							Ok::<Value, mlua::Error>(response_lua)
						}
					})?
				}
			};
			install_function_at_path(lua, &entry.path, func)?;
		}

		Ok(())
	}
}

// region:    --- Support

fn context_free_call_context() -> HandlerCallContext {
	let running = RunningContextHandle::new(RunningContext::default());
	HandlerCallContext::new(running)
}

// endregion: --- Support
