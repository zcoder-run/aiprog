//! Defines the `aip.file` write-related handlers, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.file` module provides functions for writing, appending, copying,
//! moving, deleting, and ensuring files and directories.
//!
//! ### Functions
//!
//! - `aip.file.write(params: { path: string, content: string, base_dir?: string, trim_start?: boolean, trim_end?: boolean, single_trailing_newline?: boolean }) -> FileInfo`
//! - `aip.file.append(params: { path: string, content: string, base_dir?: string }) -> FileInfo`
//! - `aip.file.copy(params: { src: string, dest: string, base_dir?: string, overwrite?: boolean }) -> FileInfo`
//! - `aip.file.move(params: { src: string, dest: string, base_dir?: string, overwrite?: boolean }) -> FileInfo`
//! - `aip.file.delete(params: { path: string, base_dir?: string }) -> boolean`
//! - `aip.file.ensure_exists(params: { path: string, content?: string, base_dir?: string, content_when_empty?: boolean }) -> FileInfo`
//! - `aip.file.ensure_dir(params: { path: string, base_dir?: string }) -> boolean`
//!
//! ---
//!
use super::file_types::FileInfo;
use super::file_types::DirContext;
use super::support::{self, aip_file_error, file_info_from_meta};
use crate::{AipFromLua, AipIntoLua, LuaExt};
use crate::{AipOutput, AipParams};
use crate::{AipRegistry, AipRegistryBuilder};
use crate::{HandlerCallContext, HandlerResult};
use crate::register_handler;
use aiprog_macros::aip_handler;
use mlua::{Lua, Value};

// region:    --- aip.file.write

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileWriteParams {
	pub path: String,
	pub content: String,
	pub base_dir: Option<String>,
	pub trim_start: Option<bool>,
	pub trim_end: Option<bool>,
	pub single_trailing_newline: Option<bool>,
}

impl AipFromLua for AipFileWriteParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let content = required_string(table, "content")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		let trim_start = table.x_try_get_bool("trim_start")?;
		let trim_end = table.x_try_get_bool("trim_end")?;
		let single_trailing_newline = table.x_try_get_bool("single_trailing_newline")?;
		Ok(AipFileWriteParams {
			path,
			content,
			base_dir,
			trim_start,
			trim_end,
			single_trailing_newline,
		})
	}
}

impl AipParams for AipFileWriteParams {}

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileWriteOutput(pub FileInfo);

impl AipIntoLua for AipFileWriteOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		support::file_info_into_lua(self.0, lua)
	}
}

impl AipOutput for AipFileWriteOutput {}

// endregion: --- aip.file.write

// region:    --- aip.file.append

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileAppendParams {
	pub path: String,
	pub content: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileAppendParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let content = required_string(table, "content")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileAppendParams {
			path,
			content,
			base_dir,
		})
	}
}

impl AipParams for AipFileAppendParams {}

// endregion: --- aip.file.append

// region:    --- aip.file.copy

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileCopyParams {
	pub src: String,
	pub dest: String,
	pub base_dir: Option<String>,
	pub overwrite: Option<bool>,
}

impl AipFromLua for AipFileCopyParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let src = required_string(table, "src")?;
		let dest = required_string(table, "dest")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		let overwrite = table.x_try_get_bool("overwrite")?;
		Ok(AipFileCopyParams {
			src,
			dest,
			base_dir,
			overwrite,
		})
	}
}

impl AipParams for AipFileCopyParams {}

// endregion: --- aip.file.copy

// region:    --- aip.file.move

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileMoveParams {
	pub src: String,
	pub dest: String,
	pub base_dir: Option<String>,
	pub overwrite: Option<bool>,
}

impl AipFromLua for AipFileMoveParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let src = required_string(table, "src")?;
		let dest = required_string(table, "dest")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		let overwrite = table.x_try_get_bool("overwrite")?;
		Ok(AipFileMoveParams {
			src,
			dest,
			base_dir,
			overwrite,
		})
	}
}

impl AipParams for AipFileMoveParams {}

// endregion: --- aip.file.move

// region:    --- aip.file.delete

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileDeleteParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileDeleteParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileDeleteParams { path, base_dir })
	}
}

impl AipParams for AipFileDeleteParams {}

// endregion: --- aip.file.delete

// region:    --- aip.file.ensure_exists

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileEnsureExistsParams {
	pub path: String,
	pub content: Option<String>,
	pub base_dir: Option<String>,
	pub content_when_empty: Option<bool>,
}

impl AipFromLua for AipFileEnsureExistsParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let content = table.x_try_get_string("content")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		let content_when_empty = table.x_try_get_bool("content_when_empty")?;
		Ok(AipFileEnsureExistsParams {
			path,
			content,
			base_dir,
			content_when_empty,
		})
	}
}

impl AipParams for AipFileEnsureExistsParams {}

// endregion: --- aip.file.ensure_exists

// region:    --- aip.file.ensure_dir

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileEnsureDirParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileEnsureDirParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileEnsureDirParams { path, base_dir })
	}
}

impl AipParams for AipFileEnsureDirParams {}

// endregion: --- aip.file.ensure_dir

// region:    --- Shared Outputs

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileBoolOutput(pub bool);

impl AipIntoLua for AipFileBoolOutput {
	fn into_lua(self, _lua: &Lua) -> crate::Result<Value> {
		Ok(Value::Boolean(self.0))
	}
}

impl AipOutput for AipFileBoolOutput {}

// endregion: --- Shared Outputs

// region:    --- Handler functions

/// Writes text content to a target file, creating parent directories as needed.
#[aip_handler]
fn aip_file_write_handler(
	call: HandlerCallContext,
	params: AipFileWriteParams,
) -> HandlerResult<AipFileWriteOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if let Some(parent) = resolved.path().parent() {
		std::fs::create_dir_all(parent.as_str()).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	let content = format_content(
		params.content,
		params.trim_start,
		params.trim_end,
		params.single_trailing_newline,
	);

	std::fs::write(resolved.path().as_str(), content)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(resolved.path(), true, resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Appends text content to a target file, creating the file and parent directories if they do not exist.
#[aip_handler]
fn aip_file_append_handler(
	call: HandlerCallContext,
	params: AipFileAppendParams,
) -> HandlerResult<AipFileWriteOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if let Some(parent) = resolved.path().parent() {
		std::fs::create_dir_all(parent.as_str()).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	use std::io::Write;
	let mut file = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(resolved.path().as_str())
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	file.write_all(params.content.as_bytes())
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(resolved.path(), true, resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Copies a file from source to destination.
#[aip_handler]
fn aip_file_copy_handler(
	call: HandlerCallContext,
	params: AipFileCopyParams,
) -> HandlerResult<AipFileWriteOutput> {
	let src_resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_read(&params.src, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if !src_resolved.path().is_file() {
		return Err(aip_file_error(
			"FILE_NOT_FOUND",
			&format!("Source is not a file: {}", src_resolved.path().as_str()),
		));
	}

	let dest_resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.dest, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	let overwrite = params.overwrite.unwrap_or(false);
	if dest_resolved.path().exists() && !overwrite {
		return Err(aip_file_error(
			"ALREADY_EXISTS",
			&format!("Destination file already exists: {}", dest_resolved.path().as_str()),
		));
	}

	if let Some(parent) = dest_resolved.path().parent() {
		std::fs::create_dir_all(parent.as_str()).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	std::fs::copy(src_resolved.path().as_str(), dest_resolved.path().as_str())
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(dest_resolved.path(), true, dest_resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Moves or renames a file from source to destination.
#[aip_handler]
fn aip_file_move_handler(
	call: HandlerCallContext,
	params: AipFileMoveParams,
) -> HandlerResult<AipFileWriteOutput> {
	let src_resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.src, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if !src_resolved.path().is_file() {
		return Err(aip_file_error(
			"FILE_NOT_FOUND",
			&format!("Source is not a file: {}", src_resolved.path().as_str()),
		));
	}

	let dest_resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.dest, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	let overwrite = params.overwrite.unwrap_or(false);
	if dest_resolved.path().exists() && !overwrite {
		return Err(aip_file_error(
			"ALREADY_EXISTS",
			&format!("Destination file already exists: {}", dest_resolved.path().as_str()),
		));
	}

	if let Some(parent) = dest_resolved.path().parent() {
		std::fs::create_dir_all(parent.as_str()).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	if std::fs::rename(src_resolved.path().as_str(), dest_resolved.path().as_str()).is_err() {
		std::fs::copy(src_resolved.path().as_str(), dest_resolved.path().as_str())
			.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
		std::fs::remove_file(src_resolved.path().as_str())
			.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	let info = file_info_from_meta(dest_resolved.path(), true, dest_resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Deletes a file from disk if it exists.
#[aip_handler]
fn aip_file_delete_handler(
	call: HandlerCallContext,
	params: AipFileDeleteParams,
) -> HandlerResult<AipFileBoolOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if resolved.path().exists() {
		if resolved.path().is_dir() {
			std::fs::remove_dir_all(resolved.path().as_str())
				.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
		} else {
			std::fs::remove_file(resolved.path().as_str())
				.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
		}
		Ok(AipFileBoolOutput(true))
	} else {
		Ok(AipFileBoolOutput(false))
	}
}

/// Ensures a target file exists, optionally writing initial content when missing or empty.
#[aip_handler]
fn aip_file_ensure_exists_handler(
	call: HandlerCallContext,
	params: AipFileEnsureExistsParams,
) -> HandlerResult<AipFileWriteOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	let target_path = resolved.path().as_str();
	let default_content = params.content.as_deref().unwrap_or("");

	if resolved.path().exists() {
		if !resolved.path().is_file() {
			return Err(aip_file_error(
				"ALREADY_EXISTS",
				&format!("Path exists and is not a file: {target_path}"),
			));
		}

		let content_when_empty = params.content_when_empty.unwrap_or(false);
		if content_when_empty {
			let meta = resolved
				.path()
				.meta()
				.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
			if meta.size == 0 && !default_content.is_empty() {
				std::fs::write(target_path, default_content)
					.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
			}
		}
	} else {
		if let Some(parent) = resolved.path().parent() {
			std::fs::create_dir_all(parent.as_str()).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
		}
		std::fs::write(target_path, default_content)
			.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
	}

	let info = file_info_from_meta(resolved.path(), true, resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Ensures a directory exists, creating missing parent folders if necessary.
#[aip_handler]
fn aip_file_ensure_dir_handler(
	call: HandlerCallContext,
	params: AipFileEnsureDirParams,
) -> HandlerResult<AipFileBoolOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_write(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	let target_path = resolved.path().as_str();
	if resolved.path().exists() {
		if resolved.path().is_dir() {
			Ok(AipFileBoolOutput(false))
		} else {
			Err(aip_file_error(
				"ALREADY_EXISTS",
				&format!("Path exists and is not a directory: {target_path}"),
			))
		}
	} else {
		std::fs::create_dir_all(target_path)
			.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
		Ok(AipFileBoolOutput(true))
	}
}

// endregion: --- Handler functions

// region:    --- Registry

/// Build and return an [`AipRegistry`] containing all write handlers for `aip.file`.
pub fn init_registry() -> crate::Result<AipRegistry> {
	let mut builder = AipRegistryBuilder::default();
	register_handler!(builder, "aip.file.write", aip_file_write_handler)?;
	register_handler!(builder, "aip.file.append", aip_file_append_handler)?;
	register_handler!(builder, "aip.file.copy", aip_file_copy_handler)?;
	register_handler!(builder, "aip.file.move", aip_file_move_handler)?;
	register_handler!(builder, "aip.file.delete", aip_file_delete_handler)?;
	register_handler!(builder, "aip.file.ensure_exists", aip_file_ensure_exists_handler)?;
	register_handler!(builder, "aip.file.ensure_dir", aip_file_ensure_dir_handler)?;
	Ok(builder.build())
}

// endregion: --- Handler functions

// region:    --- Text Formatting Helper

/// Applies string formatting options to the given content string.
#[allow(unused)]
pub fn format_content(
	mut content: String,
	trim_start: Option<bool>,
	trim_end: Option<bool>,
	single_trailing_newline: Option<bool>,
) -> String {
	if trim_start.unwrap_or(false) {
		content = crate::support::text::trim_start_if_needed(content);
	}
	if trim_end.unwrap_or(false) {
		content = crate::support::text::trim_end_if_needed(content);
	}
	if single_trailing_newline.unwrap_or(false) {
		content = crate::support::text::ensure_single_trailing_newline(content);
	}
	content
}

// endregion: --- Text Formatting Helper

// region:    --- Support: Lua Value Helpers

/// Extract the params table from a Lua value, failing with the actual type on mismatch.
fn params_table(value: &Value) -> crate::Result<&mlua::Table> {
	value.as_table().ok_or_else(|| {
		crate::Error::custom(format!(
			"Params expected to be a table, but was of type '{}'",
			value.type_name()
		))
	})
}

/// Get a required string property from a params table, failing loudly on wrong type or absence.
fn required_string(table: &mlua::Table, key: &str) -> crate::Result<String> {
	table
		.x_try_get_string(key)?
		.ok_or_else(|| crate::Error::custom(format!("Missing required property '{key}' of type 'string'")))
}

// endregion: --- Support: Lua Value Helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use mlua::Lua;

	#[test]
	fn test_format_content_variations() {
		let input = "  \nhello world\n\n\n".to_string();
		let formatted = format_content(input, Some(true), Some(true), Some(true));
		assert_eq!(formatted, "hello world\n");
	}

	#[test]
	fn test_write_params_from_lua() -> crate::Result<()> {
		let lua = Lua::new();
		let table = lua.create_table()?;
		table.set("path", "foo/bar.txt")?;
		table.set("content", "hello")?;
		table.set("trim_start", true)?;
		table.set("single_trailing_newline", true)?;

		let params = AipFileWriteParams::from_lua(&lua, Value::Table(table))?;
		assert_eq!(params.path, "foo/bar.txt");
		assert_eq!(params.content, "hello");
		assert_eq!(params.trim_start, Some(true));
		assert_eq!(params.trim_end, None);
		assert_eq!(params.single_trailing_newline, Some(true));
		Ok(())
	}

	#[test]
	fn test_copy_move_params_from_lua() -> crate::Result<()> {
		let lua = Lua::new();
		let table = lua.create_table()?;
		table.set("src", "a.txt")?;
		table.set("dest", "b.txt")?;
		table.set("overwrite", true)?;

		let copy_params = AipFileCopyParams::from_lua(&lua, Value::Table(table.clone()))?;
		assert_eq!(copy_params.src, "a.txt");
		assert_eq!(copy_params.dest, "b.txt");
		assert_eq!(copy_params.overwrite, Some(true));

		let move_params = AipFileMoveParams::from_lua(&lua, Value::Table(table))?;
		assert_eq!(move_params.src, "a.txt");
		assert_eq!(move_params.dest, "b.txt");
		assert_eq!(move_params.overwrite, Some(true));
		Ok(())
	}

	#[test]
	fn test_delete_params_from_lua() -> crate::Result<()> {
		let lua = Lua::new();
		let table = lua.create_table()?;
		table.set("path", "file.txt")?;
		table.set("base_dir", "/tmp")?;

		let params = AipFileDeleteParams::from_lua(&lua, Value::Table(table))?;
		assert_eq!(params.path, "file.txt");
		assert_eq!(params.base_dir, Some("/tmp".to_string()));
		Ok(())
	}

	#[test]
	fn test_write_append_delete_handlers() -> crate::Result<()> {
		let temp_dir = std::env::temp_dir().join(format!("aiprog_test_write_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
		std::fs::create_dir_all(&temp_dir)?;

		let dir_ctx = DirContext::from_base_dir(temp_dir.to_str().unwrap())
			.map_err(|e| crate::Error::custom(e.to_string()))?;

		let mut running_ctx = crate::running_context::RunningContext::default();
		running_ctx.insert(dir_ctx);
		let handle = crate::running_context::RunningContextHandle::new(running_ctx);
		let call = HandlerCallContext::new(handle);

		// Write
		let write_params = AipFileWriteParams {
			path: "sub/test.txt".to_string(),
			content: "  initial content\n".to_string(),
			base_dir: None,
			trim_start: Some(true),
			trim_end: None,
			single_trailing_newline: Some(true),
		};
		let write_out = aip_file_write_handler(call.clone(), write_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(write_out.0.name, "test.txt");
		assert_eq!(write_out.0.path, "sub/test.txt");

		let read_content = std::fs::read_to_string(temp_dir.join("sub/test.txt"))?;
		assert_eq!(read_content, "initial content\n");

		// Append
		let append_params = AipFileAppendParams {
			path: "sub/test.txt".to_string(),
			content: "appended line\n".to_string(),
			base_dir: None,
		};
		let append_out = aip_file_append_handler(call.clone(), append_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(append_out.0.name, "test.txt");

		let read_after_append = std::fs::read_to_string(temp_dir.join("sub/test.txt"))?;
		assert_eq!(read_after_append, "initial content\nappended line\n");

		// Delete existing
		let delete_params = AipFileDeleteParams {
			path: "sub/test.txt".to_string(),
			base_dir: None,
		};
		let delete_out = aip_file_delete_handler(call.clone(), delete_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert!(delete_out.0);
		assert!(!temp_dir.join("sub/test.txt").exists());

		// Delete non-existing
		let delete_params_missing = AipFileDeleteParams {
			path: "sub/test.txt".to_string(),
			base_dir: None,
		};
		let delete_missing_out = aip_file_delete_handler(call, delete_params_missing).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert!(!delete_missing_out.0);

		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}

	#[test]
	fn test_copy_and_move_handlers() -> crate::Result<()> {
		let temp_dir = std::env::temp_dir().join(format!("aiprog_test_copy_move_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
		std::fs::create_dir_all(&temp_dir)?;

		let dir_ctx = DirContext::from_base_dir(temp_dir.to_str().unwrap())
			.map_err(|e| crate::Error::custom(e.to_string()))?;

		let mut running_ctx = crate::running_context::RunningContext::default();
		running_ctx.insert(dir_ctx);
		let handle = crate::running_context::RunningContextHandle::new(running_ctx);
		let call = HandlerCallContext::new(handle);

		// Prepare source file
		let src_file = temp_dir.join("origin.txt");
		std::fs::write(&src_file, "original content")?;

		// Copy without overwrite
		let copy_params = AipFileCopyParams {
			src: "origin.txt".to_string(),
			dest: "nested/copy.txt".to_string(),
			base_dir: None,
			overwrite: Some(false),
		};
		let copy_out = aip_file_copy_handler(call.clone(), copy_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(copy_out.0.name, "copy.txt");
		assert_eq!(copy_out.0.path, "nested/copy.txt");
		assert_eq!(std::fs::read_to_string(temp_dir.join("nested/copy.txt"))?, "original content");
		assert!(src_file.exists());

		// Copy conflict without overwrite fails
		let copy_conflict_params = AipFileCopyParams {
			src: "origin.txt".to_string(),
			dest: "nested/copy.txt".to_string(),
			base_dir: None,
			overwrite: Some(false),
		};
		let copy_err = aip_file_copy_handler(call.clone(), copy_conflict_params);
		assert!(copy_err.is_err());

		// Copy conflict with overwrite succeeds
		std::fs::write(&src_file, "updated content")?;
		let copy_overwrite_params = AipFileCopyParams {
			src: "origin.txt".to_string(),
			dest: "nested/copy.txt".to_string(),
			base_dir: None,
			overwrite: Some(true),
		};
		let copy_overwrite_out = aip_file_copy_handler(call.clone(), copy_overwrite_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(copy_overwrite_out.0.name, "copy.txt");
		assert_eq!(std::fs::read_to_string(temp_dir.join("nested/copy.txt"))?, "updated content");

		// Move file
		let move_params = AipFileMoveParams {
			src: "origin.txt".to_string(),
			dest: "moved/target.txt".to_string(),
			base_dir: None,
			overwrite: Some(false),
		};
		let move_out = aip_file_move_handler(call.clone(), move_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(move_out.0.name, "target.txt");
		assert_eq!(move_out.0.path, "moved/target.txt");
		assert_eq!(std::fs::read_to_string(temp_dir.join("moved/target.txt"))?, "updated content");
		assert!(!src_file.exists());

		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}

	#[test]
	fn test_ensure_exists_and_ensure_dir_handlers() -> crate::Result<()> {
		let temp_dir = std::env::temp_dir().join(format!("aiprog_test_ensure_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
		std::fs::create_dir_all(&temp_dir)?;

		let dir_ctx = DirContext::from_base_dir(temp_dir.to_str().unwrap())
			.map_err(|e| crate::Error::custom(e.to_string()))?;

		let mut running_ctx = crate::running_context::RunningContext::default();
		running_ctx.insert(dir_ctx);
		let handle = crate::running_context::RunningContextHandle::new(running_ctx);
		let call = HandlerCallContext::new(handle);

		// Ensure dir creates new directory hierarchy
		let ensure_dir_params = AipFileEnsureDirParams {
			path: "nested/a/b".to_string(),
			base_dir: None,
		};
		let created_dir = aip_file_ensure_dir_handler(call.clone(), ensure_dir_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert!(created_dir.0);
		assert!(temp_dir.join("nested/a/b").is_dir());

		// Ensure dir returns false when already present
		let ensure_dir_again_params = AipFileEnsureDirParams {
			path: "nested/a/b".to_string(),
			base_dir: None,
		};
		let created_dir_again = aip_file_ensure_dir_handler(call.clone(), ensure_dir_again_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert!(!created_dir_again.0);

		// Ensure exists creates missing file with initial content
		let ensure_file_params = AipFileEnsureExistsParams {
			path: "nested/a/b/file.txt".to_string(),
			content: Some("initial content".to_string()),
			base_dir: None,
			content_when_empty: None,
		};
		let file_out = aip_file_ensure_exists_handler(call.clone(), ensure_file_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(file_out.0.name, "file.txt");
		assert_eq!(std::fs::read_to_string(temp_dir.join("nested/a/b/file.txt"))?, "initial content");

		// Ensure exists on non-empty file does not overwrite content
		let ensure_existing_file_params = AipFileEnsureExistsParams {
			path: "nested/a/b/file.txt".to_string(),
			content: Some("overwritten content".to_string()),
			base_dir: None,
			content_when_empty: Some(true),
		};
		let file_existing_out = aip_file_ensure_exists_handler(call.clone(), ensure_existing_file_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(file_existing_out.0.name, "file.txt");
		assert_eq!(std::fs::read_to_string(temp_dir.join("nested/a/b/file.txt"))?, "initial content");

		// Ensure exists on empty file fills content when content_when_empty is true
		let empty_file_path = temp_dir.join("nested/a/b/empty.txt");
		std::fs::write(&empty_file_path, "")?;
		let ensure_empty_file_params = AipFileEnsureExistsParams {
			path: "nested/a/b/empty.txt".to_string(),
			content: Some("populated content".to_string()),
			base_dir: None,
			content_when_empty: Some(true),
		};
		let file_empty_out = aip_file_ensure_exists_handler(call, ensure_empty_file_params).map_err(|e| crate::Error::custom(e.to_string()))?;
		assert_eq!(file_empty_out.0.name, "empty.txt");
		assert_eq!(std::fs::read_to_string(empty_file_path)?, "populated content");

		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}

	#[test]
	fn test_write_init_registry() -> crate::Result<()> {
		let registry = init_registry()?;
		let handlers = registry.list_registered_fns();
		assert!(handlers.iter().any(|f| f.path == "aip.file.write"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.append"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.copy"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.move"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.delete"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.ensure_exists"));
		assert!(handlers.iter().any(|f| f.path == "aip.file.ensure_dir"));

		let merged = crate::modules::aip_file::register::init_registry()?;
		let merged_handlers = merged.list_registered_fns();
		assert!(merged_handlers.iter().any(|f| f.path == "aip.file.read"));
		assert!(merged_handlers.iter().any(|f| f.path == "aip.file.write"));
		Ok(())
	}
}

// endregion: --- Tests
