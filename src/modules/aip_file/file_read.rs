//! Defines the `aip.file` read-related handlers, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.file` module provides functions for reading and listing files.
//!
//! ### Functions
//!
//! - `aip.file.read(params: { path: string, base_dir?: string }) -> { info: FileInfo, content: string }`
//! - `aip.file.list(params: { globs: string | string[], base_dir?: string, absolute?: boolean, with_meta?: boolean }) -> FileInfo[]`
//! - `aip.file.list_read(params: { globs: string | string[], base_dir?: string, absolute?: boolean, with_meta?: boolean }) -> FileRecord[]`
//! - `aip.file.first(params: { globs: string | string[], base_dir?: string, absolute?: boolean, with_meta?: boolean }) -> FileInfo | nil`
//! - `aip.file.info(params: { path: string, base_dir?: string }) -> FileInfo | nil`
//! - `aip.file.exists(params: { path: string, base_dir?: string }) -> boolean`
//! - `aip.file.stats(params: { globs?: string | string[], base_dir?: string }) -> FileStats | nil`
//!
//! ---
//!
use super::file_types::{DirContext, FileInfo, FileRecord, FileStats};
use super::support::{self, aip_file_error, file_info_from_meta, list_files_matching, validate_glob_patterns};
use crate::{AipFromLua, AipIntoLua, HandlerCallContext, HandlerResult, LuaExt};
use crate::{AipOutput, AipParams};
use crate::{AipRegistry, AipRegistryBuilder};
use aiprog_macros::{aip_handler, register_handler};
use mlua::{Lua, Value};

/// Register all read-related handlers into the given `AipRegistry`.
///
/// This function captures the provided `FileContext` and wraps each handler so
/// that it receives the context automatically.
pub fn init_registry() -> crate::Result<AipRegistry> {
	let mut registry = AipRegistryBuilder::default();

	register_handler!(registry, "aip.file.read", aip_file_read_handler)?;
	register_handler!(registry, "aip.file.list", aip_file_list_handler)?;
	register_handler!(registry, "aip.file.list_read", aip_file_list_read_handler)?;
	register_handler!(registry, "aip.file.first", aip_file_first_handler)?;
	register_handler!(registry, "aip.file.info", aip_file_info_handler)?;
	register_handler!(registry, "aip.file.exists", aip_file_exists_handler)?;
	register_handler!(registry, "aip.file.stats", aip_file_stats_handler)?;

	Ok(registry.build())
}

// region:    --- aip.file.read

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileReadParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileReadParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileReadParams { path, base_dir })
	}
}

impl AipParams for AipFileReadParams {}

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileReadOutput(pub FileRecord);

impl AipIntoLua for AipFileReadOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		support::file_record_into_lua(self.0, lua)
	}
}

impl AipOutput for AipFileReadOutput {}

// endregion: --- aip.file.read

// region:    --- AipFileListParams

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileListParams {
	/// Glob pattern or list of glob patterns to match files.
	/// Supports negative globs prefixed with `!` (e.g., `!some-dir/*.*`), special to this API.
	pub globs: FileGlobs,

	pub base_dir: Option<String>,
	pub absolute: Option<bool>,
	pub with_meta: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum FileGlobs {
	Single(String),
	Many(Vec<String>),
}

impl FileGlobs {
	pub fn into_vec(self) -> Vec<String> {
		match self {
			FileGlobs::Single(s) => vec![s],
			FileGlobs::Many(v) => v,
		}
	}
}

impl AipFromLua for AipFileListParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;

		let globs = lua_value_to_file_globs(table, "globs")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		let absolute = table.x_try_get_bool("absolute")?;
		let with_meta = table.x_try_get_bool("with_meta")?;

		Ok(AipFileListParams {
			globs,
			base_dir,
			absolute,
			with_meta,
		})
	}
}

impl AipParams for AipFileListParams {}

// endregion: --- AipFileListParams

// region:    --- AipFileListOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileListOutput(pub Vec<FileInfo>);

impl AipIntoLua for AipFileListOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		let data_table = lua.create_table()?;
		for (i, info) in self.0.into_iter().enumerate() {
			let info_lua = support::file_info_into_lua(info, lua)?;
			data_table.set(i + 1, info_lua)?;
		}
		Ok(Value::Table(data_table))
	}
}

impl AipOutput for AipFileListOutput {}

// endregion: --- AipFileListOutput

// region:    --- AipFileListReadOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileListReadOutput(pub Vec<FileRecord>);

impl AipIntoLua for AipFileListReadOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		let data_table = lua.create_table()?;
		for (i, record) in self.0.into_iter().enumerate() {
			let record_lua = support::file_record_into_lua(record, lua)?;
			data_table.set(i + 1, record_lua)?;
		}
		Ok(Value::Table(data_table))
	}
}

impl AipOutput for AipFileListReadOutput {}

// endregion: --- AipFileListReadOutput

// region:    --- AipFileInfoParams

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileInfoParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileInfoParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileInfoParams { path, base_dir })
	}
}

impl AipParams for AipFileInfoParams {}

// endregion: --- AipFileInfoParams

// region:    --- AipFileInfoOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileInfoOutput(pub Option<FileInfo>);

impl AipIntoLua for AipFileInfoOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		match self.0 {
			Some(info) => support::file_info_into_lua(info, lua),
			None => Ok(Value::Nil),
		}
	}
}

impl AipOutput for AipFileInfoOutput {}

// endregion: --- AipFileInfoOutput

// region:    --- AipFileExistsParams

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileExistsParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileExistsParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let path = required_string(table, "path")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileExistsParams { path, base_dir })
	}
}

impl AipParams for AipFileExistsParams {}

// endregion: --- AipFileExistsParams

// region:    --- AipFileExistsOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileExistsOutput(pub bool);

impl AipIntoLua for AipFileExistsOutput {
	fn into_lua(self, _lua: &Lua) -> crate::Result<Value> {
		Ok(Value::Boolean(self.0))
	}
}

impl AipOutput for AipFileExistsOutput {}

// endregion: --- AipFileExistsOutput

// region:    --- AipFileFirstOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileFirstOutput(pub Option<FileInfo>);

impl AipIntoLua for AipFileFirstOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		match self.0 {
			Some(info) => support::file_info_into_lua(info, lua),
			None => Ok(Value::Nil),
		}
	}
}

impl AipOutput for AipFileFirstOutput {}

// endregion: --- AipFileFirstOutput

// region:    --- AipFileStatsParams

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileStatsParams {
	pub globs: Option<FileGlobs>,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileStatsParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let globs = lua_value_to_optional_file_globs(table, "globs")?;
		let base_dir = table.x_try_get_string("base_dir")?;
		Ok(AipFileStatsParams { globs, base_dir })
	}
}

impl AipParams for AipFileStatsParams {}

// endregion: --- AipFileStatsParams

// region:    --- AipFileStatsOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileStatsOutput(pub Option<FileStats>);

impl AipIntoLua for AipFileStatsOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<Value> {
		match self.0 {
			Some(stats) => support::file_stats_into_lua(&stats, lua),
			None => Ok(Value::Nil),
		}
	}
}

impl AipOutput for AipFileStatsOutput {}

// endregion: --- AipFileStatsOutput

// region:    --- Handler functions

/// Reads a file from disk and returns its content and metadata.
#[aip_handler]
fn aip_file_read_handler(call: HandlerCallContext, params: AipFileReadParams) -> HandlerResult<AipFileReadOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_read(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	if !resolved.path().is_file() {
		return Err(aip_file_error(
			"FILE_NOT_FOUND",
			&format!("Path is not a file: {}", resolved.path().as_str()),
		));
	}

	let content =
		support::read_file_content(resolved.path()).map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(resolved.path(), true, resolved.root(), false)
		.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

	let record = FileRecord { info, content };
	Ok(AipFileReadOutput(record))
}

/// Will list the files for the given file globs
/// Lists files matching the given glob patterns, returning metadata for each.
///
/// Negative globs prefixed with `!` (e.g., `!**/*.tmp`) are supported to exclude matching files, special to this API.
#[aip_handler]
fn aip_file_list_handler(call: HandlerCallContext, params: AipFileListParams) -> HandlerResult<AipFileListOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let with_meta = params.with_meta.unwrap_or(true);
	let absolute = params.absolute.unwrap_or(false);

	let paths = call.with::<DirContext, _>(|dir| list_files_matching(&globs, params.base_dir.as_deref(), dir))??;

	let mut infos: Vec<FileInfo> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(p.path(), with_meta, p.root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;
		infos.push(info);
	}

	Ok(AipFileListOutput(infos))
}

/// Will list and read the files for the given file globs
/// Lists files matching the given globs, returning both metadata and content.
#[aip_handler]
fn aip_file_list_read_handler(
	call: HandlerCallContext,
	params: AipFileListParams,
) -> HandlerResult<AipFileListReadOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = call.with::<DirContext, _>(|dir| list_files_matching(&globs, params.base_dir.as_deref(), dir))??;

	let mut records: Vec<FileRecord> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(p.path(), params.with_meta.unwrap_or(true), p.root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

		let content = support::read_file_content(p.path())
			.map_err(|e| aip_file_error("READ_FAILED", &format!("Failed reading {}: {e}", p.path().as_str())))?;

		records.push(FileRecord { info, content });
	}

	Ok(AipFileListReadOutput(records))
}

/// Returns metadata for a single file, or nil if the file does not exist.
#[aip_handler]
fn aip_file_info_handler(call: HandlerCallContext, params: AipFileInfoParams) -> HandlerResult<AipFileInfoOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_read_target(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;

	let data = if resolved.path().exists() && resolved.path().is_file() {
		Some(
			file_info_from_meta(resolved.path(), true, resolved.root(), false)
				.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?,
		)
	} else {
		None
	};

	Ok(AipFileInfoOutput(data))
}

/// Checks whether a file exists at the given path.
#[aip_handler]
fn aip_file_exists_handler(
	call: HandlerCallContext,
	params: AipFileExistsParams,
) -> HandlerResult<AipFileExistsOutput> {
	let resolved = call
		.with::<DirContext, _>(|dir| dir.resolve_read_target(&params.path, params.base_dir.as_deref()))?
		.map_err(|e| aip_file_error("PATH_POLICY_DENIED", &e.to_string()))?;
	let exists = resolved.path().exists();
	Ok(AipFileExistsOutput(exists))
}

/// Returns the first file matching the given glob patterns, or nil if none found.
#[aip_handler]
fn aip_file_first_handler(call: HandlerCallContext, params: AipFileListParams) -> HandlerResult<AipFileFirstOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = call.with::<DirContext, _>(|dir| list_files_matching(&globs, params.base_dir.as_deref(), dir))??;

	let data = paths
		.into_iter()
		.next()
		.map(|first| {
			file_info_from_meta(first.path(), params.with_meta.unwrap_or(true), first.root(), absolute)
				.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))
		})
		.transpose()?;

	Ok(AipFileFirstOutput(data))
}

/// Returns aggregate statistics (count, total size, first/last created/modified timestamps) for files matching the given globs.
#[aip_handler]
fn aip_file_stats_handler(call: HandlerCallContext, params: AipFileStatsParams) -> HandlerResult<AipFileStatsOutput> {
	let globs = match params.globs {
		Some(g) => {
			let v = g.into_vec();
			if v.is_empty() {
				return Ok(AipFileStatsOutput(None));
			}
			v
		}
		None => return Ok(AipFileStatsOutput(None)),
	};
	validate_glob_patterns(&globs)?;

	let paths = call.with::<DirContext, _>(|dir| list_files_matching(&globs, params.base_dir.as_deref(), dir))??;

	let mut number_of_files: usize = 0;
	let mut total_size: u64 = 0;
	let mut ctime_first: Option<i64> = None;
	let mut ctime_last: Option<i64> = None;
	let mut mtime_first: Option<i64> = None;
	let mut mtime_last: Option<i64> = None;

	for p in paths {
		let info = file_info_from_meta(p.path(), true, p.root(), false)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

		number_of_files += 1;
		if let Some(s) = info.size {
			total_size += s;
		}
		if let Some(ct) = info.ctime {
			ctime_first = Some(ctime_first.map_or(ct, |v| v.min(ct)));
			ctime_last = Some(ctime_last.map_or(ct, |v| v.max(ct)));
		}
		if let Some(mt) = info.mtime {
			mtime_first = Some(mtime_first.map_or(mt, |v| v.min(mt)));
			mtime_last = Some(mtime_last.map_or(mt, |v| v.max(mt)));
		}
	}

	Ok(AipFileStatsOutput(Some(FileStats {
		number_of_files,
		total_size,
		ctime_first,
		ctime_last,
		mtime_first,
		mtime_last,
	})))
}

// endregion: --- Handler functions

// region:    --- Support: Lua value helpers

fn lua_value_to_file_globs(table: &mlua::Table, key: &str) -> crate::Result<FileGlobs> {
	let val: Value = table.get(key)?;
	if let Some(s) = val.x_as_lua_str() {
		Ok(FileGlobs::Single(s.to_string()))
	} else if let Some(list) = val.x_as_list() {
		let mut vec = Vec::new();
		for v in &list {
			let s = v.x_as_lua_str().ok_or_else(|| {
				crate::Error::custom(format!(
					"Property '{key}' entries expected to be of type 'string', but got type '{}'",
					v.type_name()
				))
			})?;
			vec.push(s.to_string());
		}
		if vec.is_empty() {
			return Err(crate::Error::custom(format!(
				"Property '{key}' must not be an empty list"
			)));
		}
		Ok(FileGlobs::Many(vec))
	} else if val.is_nil() || val.x_is_null() {
		Err(crate::Error::custom(format!(
			"Missing required property '{key}' of type 'string or string[]'"
		)))
	} else {
		Err(crate::Error::custom(format!(
			"Property '{key}' expected to be of type 'string or string[]', but was of type '{}'",
			val.type_name()
		)))
	}
}

fn lua_value_to_optional_file_globs(table: &mlua::Table, key: &str) -> crate::Result<Option<FileGlobs>> {
	let val: Value = table.get(key)?;
	if val.is_nil() || val.x_is_null() {
		return Ok(None);
	}
	lua_value_to_file_globs(table, key).map(Some)
}

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

// endregion: --- Support: Lua value helpers

// region:    --- Tests

#[cfg(test)]
#[path = "file_read_tests.rs"]
mod tests;

// endregion: --- Tests
