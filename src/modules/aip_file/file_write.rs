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
use super::file_types::DirContext;
use super::file_types::FileInfo;
use super::support::{self, aip_file_error, file_info_from_meta};
use crate::register_handler;
use crate::{AipFromLua, AipIntoLua, LuaExt};
use crate::{AipOutput, AipParams};
use crate::{AipRegistry, AipRegistryBuilder};
use crate::{HandlerCallContext, HandlerResult};
use aiprog_macros::aip_handler;
use mlua::{Lua, Value};

// region:    --- aip.file.write

/// Parameters for writing content to a file.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileWriteParams {
	/// Target file path to write to (relative to base directory or workspace).
	pub path: String,
	/// Text content to write into the file.
	pub content: String,
	/// Optional base directory from which relative paths are resolved.
	pub base_dir: Option<String>,
	/// Whether to trim leading whitespace from content before writing.
	pub trim_start: Option<bool>,
	/// Whether to trim trailing whitespace from content before writing.
	pub trim_end: Option<bool>,
	/// Whether to ensure the written content ends with a single newline character.
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

/// Output result returned by file write operations containing target file metadata.
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

/// Parameters for appending content to a file.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileAppendParams {
	/// Target file path to append to (relative to base directory or workspace).
	pub path: String,
	/// Text content to append to the file.
	pub content: String,
	/// Optional base directory from which relative paths are resolved.
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

/// Parameters for copying a file from a source location to a destination.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileCopyParams {
	/// Source file path to copy from.
	pub src: String,
	/// Destination file path to copy to.
	pub dest: String,
	/// Optional base directory from which relative paths are resolved.
	pub base_dir: Option<String>,
	/// Whether to overwrite the destination file if it already exists. Defaults to false.
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

/// Parameters for moving or renaming a file from a source location to a destination.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileMoveParams {
	/// Source file path to move.
	pub src: String,
	/// Destination file path to move to.
	pub dest: String,
	/// Optional base directory from which relative paths are resolved.
	pub base_dir: Option<String>,
	/// Whether to overwrite the destination file if it already exists. Defaults to false.
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

/// Parameters for deleting a file or directory from the filesystem.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileDeleteParams {
	/// Path of the file or directory to delete.
	pub path: String,
	/// Optional base directory from which relative paths are resolved.
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

/// Parameters for ensuring a file exists on disk, optionally providing default content.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileEnsureExistsParams {
	/// Target file path to check or create.
	pub path: String,
	/// Optional initial content to write if the file does not exist.
	pub content: Option<String>,
	/// Optional base directory from which relative paths are resolved.
	pub base_dir: Option<String>,
	/// Whether to populate content when the file exists but has zero length.
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

/// Parameters for ensuring a directory exists on disk.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileEnsureDirParams {
	/// Directory path to check or create.
	pub path: String,
	/// Optional base directory from which relative paths are resolved.
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

/// Boolean output result returned by filesystem operations such as delete and ensure_dir.
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
fn aip_file_write_handler(call: HandlerCallContext, params: AipFileWriteParams) -> HandlerResult<AipFileWriteOutput> {
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

	std::fs::write(resolved.path().as_str(), content).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(resolved.path(), true, resolved.root(), false)
		.map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;

	Ok(AipFileWriteOutput(info))
}

/// Appends text content to a target file, creating the file and parent directories if they do not exist.
#[aip_handler]
fn aip_file_append_handler(call: HandlerCallContext, params: AipFileAppendParams) -> HandlerResult<AipFileWriteOutput> {
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
fn aip_file_copy_handler(call: HandlerCallContext, params: AipFileCopyParams) -> HandlerResult<AipFileWriteOutput> {
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
fn aip_file_move_handler(call: HandlerCallContext, params: AipFileMoveParams) -> HandlerResult<AipFileWriteOutput> {
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
fn aip_file_delete_handler(call: HandlerCallContext, params: AipFileDeleteParams) -> HandlerResult<AipFileBoolOutput> {
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
		std::fs::write(target_path, default_content).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
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
		std::fs::create_dir_all(target_path).map_err(|e| aip_file_error("WRITE_FAILED", &e.to_string()))?;
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
		content = crate::base::text::trim_start_if_needed(content);
	}
	if trim_end.unwrap_or(false) {
		content = crate::base::text::trim_end_if_needed(content);
	}
	if single_trailing_newline.unwrap_or(false) {
		content = crate::base::text::ensure_single_trailing_newline(content);
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
#[path = "file_write_tests.rs"]
mod tests;

// endregion: --- Tests
