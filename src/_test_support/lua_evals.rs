use crate::{ScriptError, ScriptResult};
use mlua::{Lua, Value};

/// Process correctly the lua eval result
/// (Used by the lua engine eval, and test)
pub fn process_lua_eval_result(_lua: &Lua, res: mlua::Result<Value>, script: &str) -> ScriptResult<Value> {
	let res = match res {
		Ok(res) => res,
		Err(err) => return Err(ScriptError::from_error_with_script(&err, script)),
	};

	let res = match res {
		// This is when we d with pcall(...), see test_lua_json_parse_invalid
		Value::Error(err) => {
			return Err(ScriptError::from_error_with_script(&err, script));
			// return Err(Error::from(&*err));
		}
		res => res,
	};

	Ok(res)
}
