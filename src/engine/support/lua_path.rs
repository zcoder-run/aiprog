//! Home of the shared dotted Lua path installation logic.
//!
//! Content:
//!
//! - Installing an arbitrary Lua value at a dotted path, creating intermediate tables as needed.
//! - Installing a Lua function at a dotted path, rejecting leaf conflicts.
//!
//! Both installers share the same intermediate table resolution and non-table conflict errors.

use mlua::{Function, Lua, Table, Value};

/// Install a Lua function at a dotted path, creating intermediate tables as needed.
pub fn install_function_at_path(lua: &Lua, path: &str, func: Function) -> mlua::Result<()> {
	let (parent, leaf) = resolve_parent_table(lua, path, "Invalid empty path for function installation")?;
	// Reject targeted leaf conflicts by default
	if let Ok(existing) = parent.get::<Value>(leaf)
		&& !existing.is_nil()
	{
		return Err(mlua::Error::RuntimeError(format!(
			"Function already exists at leaf '{}' in path '{}'",
			leaf, path
		)));
	}
	parent.set(leaf, func)?;

	Ok(())
}

// region:    --- Support

/// Resolve the parent table of a dotted path, creating intermediate tables as needed,
/// and return it together with the leaf segment.
fn resolve_parent_table<'a>(lua: &Lua, path: &'a str, empty_path_msg: &str) -> mlua::Result<(Table, &'a str)> {
	let segments: Vec<&str> = path.split('.').collect();
	let Some((leaf, ancestors)) = segments.split_last() else {
		return Err(mlua::Error::RuntimeError(empty_path_msg.into()));
	};
	let globals = lua.globals();
	let mut current = globals;
	for &seg in ancestors {
		let next: Value = current.get(seg)?;
		if next.is_nil() {
			let table = lua.create_table()?;
			current.set(seg, table.clone())?;
			current = table;
		} else if let Value::Table(t) = next {
			current = t;
		} else {
			return Err(mlua::Error::RuntimeError(format!(
				"Path segment '{}' exists but is not a table",
				seg
			)));
		}
	}

	Ok((current, *leaf))
}

// endregion: --- Support
