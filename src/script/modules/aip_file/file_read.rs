use super::file_types::{FileInfo, FileRecord, FileStats};
use super::support::{
	self, FileContext, aip_file_error, file_info_from_meta, list_files_matching, validate_glob_patterns,
};
use crate::script::script_error::ScriptResult;
use crate::script::{AipApiResult, AipFromLua, AipIntoLua, LuaExt};
use mlua::{Lua, Value};

/// Register all read-related handlers into the given `AipRegistry`.
///
/// This function captures the provided `FileContext` and wraps each handler so
/// that it receives the context automatically.
pub fn register_read(registry: &mut crate::registry::AipRegistry, ctx: FileContext) -> crate::Result<()> {
	// -- aip.file.read
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.read", move |p: AipFileReadParams| {
			aip_file_read_handler(p, &ctx)
		})?;
	}

	// -- aip.file.list
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.list", move |p: AipFileListParams| {
			aip_file_list_handler(p, &ctx)
		})?;
	}

	// -- aip.file.list_read
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.list_read", move |p: AipFileListReadParams| {
			aip_file_list_read_handler(p, &ctx)
		})?;
	}

	// -- aip.file.info
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.info", move |p: AipFileInfoParams| {
			aip_file_info_handler(p, &ctx)
		})?;
	}

	// -- aip.file.exists
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.exists", move |p: AipFileExistsParams| {
			aip_file_exists_handler(p, &ctx)
		})?;
	}

	// -- aip.file.first
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.first", move |p: AipFileFirstParams| {
			aip_file_first_handler(p, &ctx)
		})?;
	}

	// -- aip.file.stats
	{
		let ctx = ctx.clone();
		registry.register_sync::<_, _, _, _>("aip.file.stats", move |p: AipFileStatsParams| {
			aip_file_stats_handler(p, &ctx)
		})?;
	}

	Ok(())
}

// region:    --- AipFileReadParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileReadParams {
	pub path: String,
	#[serde(default)]
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileReadParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		Ok(AipFileReadParams { path, base_dir })
	}
}

impl crate::script::AipParams for AipFileReadParams {}

// endregion: --- AipFileReadParams

// region:    --- AipFileReadOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileReadOutput {
	pub data: FileRecord,
}

impl AipIntoLua for AipFileReadOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let record_lua = support::file_record_into_lua(self.data, lua)?;
		table.set("data", record_lua)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileReadOutput {}

// endregion: --- AipFileReadOutput

// region:    --- AipFileListParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileListParams {
	pub globs: FileGlobs,
	#[serde(default)]
	pub base_dir: Option<String>,
	#[serde(default)]
	pub absolute: Option<bool>,
	#[serde(default)]
	pub with_meta: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
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
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;

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

impl crate::script::AipParams for AipFileListParams {}

// endregion: --- AipFileListParams

// region:    --- AipFileListOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileListOutput {
	pub data: Vec<FileInfo>,
}

impl AipIntoLua for AipFileListOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let data_table = lua.create_table()?;
		for (i, info) in self.data.into_iter().enumerate() {
			let info_lua = support::file_info_into_lua(info, lua)?;
			data_table.set(i + 1, info_lua)?;
		}
		table.set("data", data_table)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileListOutput {}

// endregion: --- AipFileListOutput

// region:    --- AipFileListReadParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileListReadParams {
	pub globs: FileGlobs,
	#[serde(default)]
	pub base_dir: Option<String>,
	#[serde(default)]
	pub absolute: Option<bool>,
}

impl AipFromLua for AipFileListReadParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let globs = lua_value_to_file_globs(table, "globs")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		let absolute: Option<bool> = table.x_get_bool("absolute");

		Ok(AipFileListReadParams {
			globs,
			base_dir,
			absolute,
		})
	}
}

impl crate::script::AipParams for AipFileListReadParams {}

// endregion: --- AipFileListReadParams

// region:    --- AipFileListReadOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileListReadOutput {
	pub data: Vec<FileRecord>,
}

impl AipIntoLua for AipFileListReadOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let data_table = lua.create_table()?;
		for (i, record) in self.data.into_iter().enumerate() {
			let record_lua = support::file_record_into_lua(record, lua)?;
			data_table.set(i + 1, record_lua)?;
		}
		table.set("data", data_table)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileListReadOutput {}

// endregion: --- AipFileListReadOutput

// region:    --- AipFileInfoParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileInfoParams {
	pub path: String,
	#[serde(default)]
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileInfoParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		Ok(AipFileInfoParams { path, base_dir })
	}
}

impl crate::script::AipParams for AipFileInfoParams {}

// endregion: --- AipFileInfoParams

// region:    --- AipFileInfoOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileInfoOutput {
	pub data: Option<FileInfo>,
}

impl AipIntoLua for AipFileInfoOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let data_lua = match self.data {
			Some(info) => support::file_info_into_lua(info, lua)?,
			None => Value::Nil,
		};
		table.set("data", data_lua)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileInfoOutput {}

// endregion: --- AipFileInfoOutput

// region:    --- AipFileExistsParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileExistsParams {
	pub path: String,
	#[serde(default)]
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileExistsParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let path: String = table.get("path")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		Ok(AipFileExistsParams { path, base_dir })
	}
}

impl crate::script::AipParams for AipFileExistsParams {}

// endregion: --- AipFileExistsParams

// region:    --- AipFileExistsOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileExistsOutput {
	pub data: bool,
}

impl AipIntoLua for AipFileExistsOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		table.set("data", self.data)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileExistsOutput {}

// endregion: --- AipFileExistsOutput

// region:    --- AipFileFirstParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileFirstParams {
	pub globs: FileGlobs,
	#[serde(default)]
	pub base_dir: Option<String>,
	#[serde(default)]
	pub absolute: Option<bool>,
}

impl AipFromLua for AipFileFirstParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let globs = lua_value_to_file_globs(table, "globs")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		let absolute: Option<bool> = table.x_get_bool("absolute");

		Ok(AipFileFirstParams {
			globs,
			base_dir,
			absolute,
		})
	}
}

impl crate::script::AipParams for AipFileFirstParams {}

// endregion: --- AipFileFirstParams

// region:    --- AipFileFirstOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileFirstOutput {
	pub data: Option<FileInfo>,
}

impl AipIntoLua for AipFileFirstOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let data_lua = match self.data {
			Some(info) => support::file_info_into_lua(info, lua)?,
			None => Value::Nil,
		};
		table.set("data", data_lua)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileFirstOutput {}

// endregion: --- AipFileFirstOutput

// region:    --- AipFileStatsParams

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipFileStatsParams {
	#[serde(default)]
	pub globs: Option<FileGlobs>,
	#[serde(default)]
	pub base_dir: Option<String>,
}

impl AipFromLua for AipFileStatsParams {
	fn from_lua(_lua: &Lua, value: Value) -> ScriptResult<Self> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::ScriptError::custom("Expected table"))?;
		let globs = lua_value_to_optional_file_globs(table, "globs")?;
		let base_dir: Option<String> = table.get("base_dir")?;
		Ok(AipFileStatsParams { globs, base_dir })
	}
}

impl crate::script::AipParams for AipFileStatsParams {}

// endregion: --- AipFileStatsParams

// region:    --- AipFileStatsOutput

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipFileStatsOutput {
	pub data: Option<FileStats>,
}

impl AipIntoLua for AipFileStatsOutput {
	fn into_lua(self, lua: &Lua) -> ScriptResult<Value> {
		let table = lua.create_table()?;
		let data_lua = match self.data {
			Some(stats) => support::file_stats_into_lua(&stats, lua)?,
			None => Value::Nil,
		};
		table.set("data", data_lua)?;
		Ok(Value::Table(table))
	}
}

impl crate::script::AipOutput for AipFileStatsOutput {}

// endregion: --- AipFileStatsOutput

// region:    --- Handler functions

fn aip_file_read_handler(params: AipFileReadParams, ctx: &FileContext) -> AipApiResult<AipFileReadOutput> {
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
	Ok(AipFileReadOutput { data: record })
}

fn aip_file_list_handler(params: AipFileListParams, ctx: &FileContext) -> AipApiResult<AipFileListOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let with_meta = params.with_meta.unwrap_or(true);
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), ctx)?;

	let mut infos: Vec<FileInfo> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(&p, with_meta, ctx.workspace_root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;
		infos.push(info);
	}

	Ok(AipFileListOutput { data: infos })
}

fn aip_file_list_read_handler(params: AipFileListReadParams, ctx: &FileContext) -> AipApiResult<AipFileListReadOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), ctx)?;

	let mut records: Vec<FileRecord> = Vec::new();
	for p in paths {
		let info = file_info_from_meta(&p, true, ctx.workspace_root(), absolute)
			.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))?;

		let content = support::read_file_content(&p)
			.map_err(|e| aip_file_error("READ_FAILED", &format!("Failed reading {}: {e}", p.as_str())))?;

		records.push(FileRecord { info, content });
	}

	Ok(AipFileListReadOutput { data: records })
}

fn aip_file_info_handler(params: AipFileInfoParams, ctx: &FileContext) -> AipApiResult<AipFileInfoOutput> {
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

	Ok(AipFileInfoOutput { data })
}

fn aip_file_exists_handler(params: AipFileExistsParams, ctx: &FileContext) -> AipApiResult<AipFileExistsOutput> {
	let resolved = ctx
		.resolve(&params.path, params.base_dir.as_deref())
		.map_err(|e| aip_file_error("PATH_RESOLUTION_FAILED", &e.to_string()))?;
	let exists = resolved.exists();
	Ok(AipFileExistsOutput { data: exists })
}

fn aip_file_first_handler(params: AipFileFirstParams, ctx: &FileContext) -> AipApiResult<AipFileFirstOutput> {
	let globs = params.globs.into_vec();
	validate_glob_patterns(&globs)?;
	let absolute = params.absolute.unwrap_or(false);

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), ctx)?;

	let data = paths
		.into_iter()
		.next()
		.map(|first| {
			file_info_from_meta(&first, true, ctx.workspace_root(), absolute)
				.map_err(|e| aip_file_error("READ_FAILED", &e.to_string()))
		})
		.transpose()?;

	Ok(AipFileFirstOutput { data })
}

fn aip_file_stats_handler(params: AipFileStatsParams, ctx: &FileContext) -> AipApiResult<AipFileStatsOutput> {
	let globs = match params.globs {
		Some(g) => {
			let v = g.into_vec();
			if v.is_empty() {
				return Ok(AipFileStatsOutput { data: None });
			}
			v
		}
		None => return Ok(AipFileStatsOutput { data: None }),
	};
	validate_glob_patterns(&globs)?;

	let paths = list_files_matching(&globs, params.base_dir.as_deref(), ctx)?;

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

	Ok(AipFileStatsOutput {
		data: Some(FileStats {
			number_of_files,
			total_size,
			ctime_first,
			ctime_last,
			mtime_first,
			mtime_last,
		}),
	})
}

// endregion: --- Handler functions

// region:    --- Support: Lua value helpers

fn lua_value_to_file_globs(table: &mlua::Table, key: &str) -> ScriptResult<FileGlobs> {
	let val: Value = table.get(key)?;
	if let Some(s) = val.x_as_lua_str() {
		Ok(FileGlobs::Single(s.to_string()))
	} else if let Some(list) = val.x_as_list() {
		let mut vec = Vec::new();
		for v in &list {
			let s = v
				.x_as_lua_str()
				.ok_or_else(|| crate::script::ScriptError::custom("globs entry must be a string"))?;
			vec.push(s.to_string());
		}
		if vec.is_empty() {
			return Err(crate::script::ScriptError::custom("globs must not be empty"));
		}
		Ok(FileGlobs::Many(vec))
	} else {
		Err(crate::script::ScriptError::custom("Expected string or table for globs"))
	}
}

fn lua_value_to_optional_file_globs(table: &mlua::Table, key: &str) -> ScriptResult<Option<FileGlobs>> {
	let val: Value = table.get(key)?;
	if val.is_nil() || val.x_is_null() {
		return Ok(None);
	}
	lua_value_to_file_globs(table, key).map(Some)
}

// endregion: --- Support: Lua value helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::registry::AipRegistry;
	use crate::script::LuaJsonExt;
	use serde_json::json;
	use tempfile::TempDir;

	type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[tokio::test]
	async fn test_read_file_ok() -> TestResult<()> {
		let tmp = TempDir::new()?;
		let file_path = tmp.path().join("hello.txt");
		std::fs::write(&file_path, "world")?;

		let lua = mlua::Lua::new();

		// Build FileContext using SPath
		let workspace =
			simple_fs::SPath::from_std_path(tmp.path()).map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
		let ctx = FileContext::new(workspace);
		let mut registry = AipRegistry::from_empty();

		// Register the single handler directly via the registry (for unit test)
		super::register_read(&mut registry, ctx)?;

		let params_lua = mlua::Value::x_from_json_value(&lua, json!({ "path": "hello.txt" }))
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

		let value = registry.call(lua.clone(), "aip.file.read", params_lua).await?;
		let back = value
			.x_to_json_value()
			.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?
			.ok_or_else(|| mlua::Error::RuntimeError("expected JSON value".to_string()))?;

		assert_eq!(back["data"]["content"], json!("world"));
		Ok(())
	}
}

// endregion: --- Tests
