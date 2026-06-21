use crate::HandlerError;
use mlua::{Lua, Value};

/// Process correctly the lua eval result
/// (Used by the lua engine eval, and test)
#[allow(dead_code)]
pub fn process_lua_eval_result(_lua: &Lua, res: mlua::Result<Value>, script: &str) -> crate::Result<Value> {
	let res = match res {
		Ok(res) => res,
		Err(err) => return Err(HandlerError::from_lua_error_with_script(&err, script).into()),
	};

	let res = match res {
		// This is when we d with pcall(...), see test_lua_json_parse_invalid
		Value::Error(err) => {
			return Err(HandlerError::from_lua_error_with_script(&err, script).into());
			// return Err(Error::from(&*err));
		}
		res => res,
	};

	Ok(res)
}
