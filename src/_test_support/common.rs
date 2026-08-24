use crate::_test_support::lua_evals::process_lua_eval_result;
use crate::AipRegistry;
use crate::LuaJsonExt;
use crate::NativeFunctionInstaller;
use crate::NativeFunctionSet;
use crate::Result;
use mlua::{Lua, Table};
use serde_json::Value;
use std::sync::Arc;

/// Sets up a Lua instance with both functions registered under `aip.` aip_name.
#[allow(dead_code)]
pub async fn setup_lua<F>(init_fn: F, sub_module: &str) -> Result<Lua>
where
	F: FnOnce(&Lua) -> Result<Table>,
{
	let lua = Lua::new();
	let globals = lua.globals();
	let aip = lua.create_table()?;

	let path_table = init_fn(&lua)?;
	// if sub_module is empty then, assume it is a table and set them one by one
	if sub_module.is_empty() {
		for pair in path_table.pairs::<String, mlua::Value>() {
			let (key, value) = pair?;
			aip.set(key, value)?;
		}
	}
	// otherwise add it in the sub module
	else {
		aip.set(sub_module, path_table)?;
	}

	globals.set("aip", &aip)?;

	Ok(lua)
}

pub fn setup_lua_engine<F>(registry_provider: F) -> crate::Result<crate::ScriptEngine>
where
	F: FnOnce() -> crate::Result<AipRegistry>,
{
	let registry = registry_provider()?;
	let web_native = crate::modules::WebModule.native_functions();
	let safe_installer: NativeFunctionInstaller = Arc::new(move |lua: &Lua| {
		let globals = lua.globals();
		if let Ok(aip) = globals.get::<Table>("aip")
			&& aip.get::<Table>("web").is_ok()
		{
			web_native.install(lua)?;
		}
		Ok(())
	});
	let native_functions = NativeFunctionSet::new(vec![safe_installer]);

	crate::ScriptEngine::builder()
		.with_registry(registry)
		.with_native_functions(native_functions)
		.build()
		.map_err(crate::Error::Engine)
}

pub async fn eval_script(engine: &crate::ScriptEngine, code: &str) -> crate::Result<serde_json::Value> {
	let outcome = engine
		.exec(code, crate::RunningContext::default())
		.await
		.map_err(crate::Error::Engine)?;
	outcome.result
}

#[allow(dead_code)]
pub fn eval_lua(lua: &Lua, code: &str) -> Result<Value> {
	let res = lua.load(code).eval::<mlua::Value>();
	let res_lua_value = process_lua_eval_result(lua, res, code)?;
	let some_value = res_lua_value.x_to_json_value()?;
	let serde_value = some_value.ok_or_else(|| crate::Error::custom("Lua value converted to nil"))?;
	Ok(serde_value)
}
