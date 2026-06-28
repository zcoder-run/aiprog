#![allow(non_snake_case)]

use crate::{AipOutput, AipParams};
use crate::{AipRegistry, aip_handler};

use super::file_types::{FileInfo, FileRecord, FileStats};
use super::support::{
	self, FileContext, aip_file_error, file_info_from_meta, list_files_matching, validate_glob_patterns,
};
use crate::{AipFromLua, AipIntoLua, HandlerResult, LuaExt};
use mlua::{Lua, Value};

/// Register all read-related handlers into the given `AipRegistry`.
///
/// This function captures the provided `FileContext` and wraps each handler so
/// that it receives the context automatically.
pub fn init_registry_with_ctx(ctx: FileContext) -> crate::Result<AipRegistry> {
	// TODO: This is not the right way to do this
	support::set_file_context(ctx);

	let mut registry = AipRegistry::from_empty();

	registry.register_handler::<AipFileReadHandler>("aip.file.read")?;
	registry.register_handler::<AipFileListHandler>("aip.file.list")?;
	registry.register_handler::<AipFileListReadHandler>("aip.file.list_read")?;
	registry.register_handler::<AipFileFirstHandler>("aip.file.first")?;
	registry.register_handler::<AipFileInfoHandler>("aip.file.info")?;
	registry.register_handler::<AipFileExistsHandler>("aip.file.exists")?;
	registry.register_handler::<AipFileStatsHandler>("aip.file.stats")?;

	Ok(registry)
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
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
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
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;

		let globs = lua_value_to_file_globs(table, "globs")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		let absolute: Option<bool> = table.x_get_bool("absolute");
		let with_meta: Option<bool> = table.x_get_bool("with_meta");

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
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
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

/// Returns true if the file path exist
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileExistsParams {
	pub path: String,
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileExistsParams {
	fn from_lua(_lua: &Lua, value: Value) -> crate::Result<Self> {
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
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
		let table = value.as_table().ok_or_else(|| crate::Error::custom("Expected table"))?;
		let globs = lua_value_to_optional_file_globs(table, "globs")?;
		let base_dir: Option<String> = table.get("base_dir")?;
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

#[aip_handler]
fn AipFileReadHandler(params: AipFileReadParams) -> HandlerResult<AipFileReadOutput> {
	let ctx = support::get_file_context();
	let resolved = ctx
		.resolve(&params.path, params.base_dir.as_deref())
		.map_err(|e| aip_file_error("PATH_RESOLUTION_FAILED", &e.to_string()))?;

	if !resolved.is_file() {
		return Err(aip_file_error(
			"FILE_NOT_FOUND",
			&format!("Path is not a file: {}", resolved.as_str()),
		));
	}

	let content = support::read_file_content(&resolved).map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

	let info = file_info_from_meta(&resolved, true, ctx.workspace_root(), false)
		.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

	let record = FileRecord { info, content };
	Ok(AipFileReadOutput(record))
}

/// Will list the files for the given file globs
#[aip_handler]
fn AipFileListHandler(params: AipFileListParams) -> HandlerResult<AipFileListOutput> {
	let ctx = support::get_file_context();
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let with_meta = params.with_meta.unwrap_or(true);
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), &ctx)?;

	let mut infos: Vec<FileInfo> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(&p, with_meta, ctx.workspace_root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;
		infos.push(info);
	}

	Ok(AipFileListOutput(infos))
}

/// Will list and read the files for the given file globs
#[aip_handler]
fn AipFileListReadHandler(params: AipFileListParams) -> HandlerResult<AipFileListReadOutput> {
	let ctx = support::get_file_context();
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), &ctx)?;

	let mut records: Vec<FileRecord> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(&p, params.with_meta.unwrap_or(true), ctx.workspace_root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

		let content = support::read_file_content(&p)
			.map_err(|e| aip_file_error("READ_FAILED", &format!("Failed reading {}: {e}", p.as_str())))?;

		records.push(FileRecord { info, content });
	}

	Ok(AipFileListReadOutput(records))
}

#[aip_handler]
fn AipFileInfoHandler(params: AipFileInfoParams) -> HandlerResult<AipFileInfoOutput> {
	let ctx = support::get_file_context();
	let resolved = ctx
		.resolve(&params.path, params.base_dir.as_deref())
		.map_err(|e| aip_file_error("PATH_RESOLUTION_FAILED", &e.to_string()))?;

	let data = if resolved.exists() && resolved.is_file() {
		Some(
			file_info_from_meta(&resolved, true, ctx.workspace_root(), false)
				.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?,
		)
	} else {
		None
	};

	Ok(AipFileInfoOutput(data))
}

#[aip_handler]
fn AipFileExistsHandler(params: AipFileExistsParams) -> HandlerResult<AipFileExistsOutput> {
	let ctx = support::get_file_context();
	let resolved = ctx
		.resolve(&params.path, params.base_dir.as_deref())
		.map_err(|e| aip_file_error("PATH_RESOLUTION_FAILED", &e.to_string()))?;
	let exists = resolved.exists();
	Ok(AipFileExistsOutput(exists))
}

#[aip_handler]
fn AipFileFirstHandler(params: AipFileListParams) -> HandlerResult<AipFileFirstOutput> {
	let ctx = support::get_file_context();
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), &ctx)?;

	let data = paths
		.into_iter()
		.next()
		.map(|first| {
			file_info_from_meta(&first, params.with_meta.unwrap_or(true), ctx.workspace_root(), absolute)
				.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))
		})
		.transpose()?;

	Ok(AipFileFirstOutput(data))
}

#[aip_handler]
fn AipFileStatsHandler(params: AipFileStatsParams) -> HandlerResult<AipFileStatsOutput> {
	let ctx = support::get_file_context();
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

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), &ctx)?;

	let mut number_of_files: usize = 0;
	let mut total_size: u64 = 0;
	let mut ctime_first: Option<i64> = None;
	let mut ctime_last: Option<i64> = None;
	let mut mtime_first: Option<i64> = None;
	let mut mtime_last: Option<i64> = None;

	for p in paths {
		let info = file_info_from_meta(&p, true, ctx.workspace_root(), false)
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
			let s = v
				.x_as_lua_str()
				.ok_or_else(|| crate::Error::custom("globs entry must be a string"))?;
			vec.push(s.to_string());
		}
		if vec.is_empty() {
			return Err(crate::Error::custom("globs must not be empty"));
		}
		Ok(FileGlobs::Many(vec))
	} else {
		Err(crate::Error::custom("Expected string or table for globs"))
	}
}

fn lua_value_to_optional_file_globs(table: &mlua::Table, key: &str) -> crate::Result<Option<FileGlobs>> {
	let val: Value = table.get(key)?;
	if val.is_nil() || val.x_is_null() {
		return Ok(None);
	}
	lua_value_to_file_globs(table, key).map(Some)
}

// endregion: --- Support: Lua value helpers

// region:    --- Tests

#[cfg(test)]
#[path = "file_read_tests.rs"]
mod tests;

// endregion: --- Tests
