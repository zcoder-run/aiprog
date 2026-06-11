//! Common support functions for the aip.file module.
//!
//! Contains path resolution, file listing using simple-fs, and shared
//! Lua conversion helpers.

use simple_fs::{SPath, read_to_string, list_files, ListOptions};
use crate::Result;
use crate::script::{AipApiError, ScriptResult};
use mlua::{Lua, Value};

use super::file_types::{FileInfo, FileRecord, FileStats};

// region:    --- FileContext

/// Holds the workspace root for path resolution.
#[derive(Debug, Clone)]
pub struct FileContext {
	workspace_root: SPath,
}

impl FileContext {
	/// Create a new context with the given workspace root.
	pub fn new(workspace_root: impl Into<SPath>) -> Self {
		Self {
			workspace_root: workspace_root.into(),
		}
	}

	/// Return a reference to the workspace root SPath.
	pub fn workspace_root(&self) -> &SPath {
		&self.workspace_root
	}

	/// Resolve a user-supplied path and optional base directory.
	///
	/// If the path is absolute, it is returned directly.
	/// Otherwise, it is resolved against the base directory if provided,
	/// or the workspace root.
	pub fn resolve(&self, path: &str, base_dir: Option<&str>) -> crate::Result<SPath> {
		let sp = SPath::new(path);
		if sp.is_absolute() {
			Ok(sp)
		} else {
			let base = match base_dir {
				Some(dir) => SPath::new(dir),
				None => self.workspace_root.clone(),
			};
			Ok(base.join(sp))
		}
	}
}

// endregion: --- FileContext

// region:    --- File listing

/// List files matching the given glob patterns.
///
/// Patterns starting with `!` are treated as exclusion patterns.
/// Returns the full, normalized `SPath` for each matched file.
pub fn list_files_matching(
	globs: &[String],
	base_dir: Option<&str>,
	ctx: &FileContext,
) -> crate::Result<Vec<SPath>> {
	// Separate include and exclude patterns.
	let mut include_strs: Vec<&str> = Vec::new();
	let mut exclude_strs: Vec<&str> = Vec::new();

	for g in globs {
		let trimmed = g.trim();
		if trimmed.is_empty() {
			continue;
		}
		if let Some(ex) = trimmed.strip_prefix('!') {
			exclude_strs.push(ex);
		} else {
			include_strs.push(trimmed);
		}
	}

	if include_strs.is_empty() {
		return Err(crate::Error::cc(
			"No include patterns specified",
			format!("globs: {:?}", globs),
		));
	}

	let dir = ctx.resolve(base_dir.unwrap_or("."), None)?;

	let opts = ListOptions::default()
		.with_relative_glob()
		.with_exclude_globs(&exclude_strs);

	let mut files = list_files(&dir, Some(&include_strs), Some(opts))
		.map_err(|e| crate::Error::cc("File listing failed", e.to_string()))?;

	// simple-fs may return paths relative to the directory; join to ensure full paths.
	files = files.into_iter().map(|f| dir.join(f)).collect();
	Ok(files)
}

// endregion: --- File listing

// region:    --- File I/O helpers

/// Read the entire content of a file as a String.
pub fn read_file_content(path: &SPath) -> crate::Result<String> {
	read_to_string(path)
		.map_err(|e| crate::Error::cc("Failed to read file", e.to_string()))
}

// endregion: --- File I/O helpers

// region:    --- File info from path

/// Build a `FileInfo` from an `SPath` and, optionally, metadata.
///
/// The `path` field of the returned `FileInfo` is set according to the
/// `absolute` flag: when `true`, it contains the canonicalized absolute
/// path; otherwise, it contains a path relative to `workspace_root`.
pub fn file_info_from_meta(
	path: &SPath,
	with_meta: bool,
	workspace_root: &SPath,
	absolute: bool,
) -> crate::Result<FileInfo> {
	let name = path
		.file_name()
		.unwrap_or_default()
		.to_string();
	let stem = path
		.file_stem()
		.unwrap_or_default()
		.to_string();
	let ext = path
		.extension()
		.unwrap_or_default()
		.to_string();

	let (size, ctime, mtime) = if with_meta {
		let meta = path
			.meta()
			.map_err(|e| crate::Error::cc("Failed to get metadata", e.to_string()))?;
		let size = meta.size;
		let ctime = Some(meta.created_epoch_us);
		let mtime = Some(meta.modified_epoch_us);
		(Some(size), ctime, mtime)
	} else {
		(None, None, None)
	};

	let path_str = if absolute {
		path.canonicalize()
			.map_err(|e| crate::Error::cc("Failed to canonicalize path", e.to_string()))?
			.as_str()
			.to_string()
	} else {
		path.diff(workspace_root)
			.unwrap_or_else(|| path.clone())
			.as_str()
			.to_string()
	};

	Ok(FileInfo {
		path: path_str,
		name,
		stem,
		ext,
		size,
		ctime,
		mtime,
	})
}

// endregion: --- File info from path

// region:    --- Lua conversion helpers

/// Convert a `FileInfo` into a Lua table.
pub fn file_info_into_lua(info: FileInfo, lua: &Lua) -> ScriptResult<Value> {
	let table = lua.create_table()?;
	table.set("path", info.path.as_str())?;
	table.set("name", info.name.as_str())?;
	table.set("stem", info.stem.as_str())?;
	table.set("ext", info.ext.as_str())?;
	if let Some(size) = info.size {
		table.set("size", size)?;
	}
	if let Some(ctime) = info.ctime {
		table.set("ctime", ctime)?;
	}
	if let Some(mtime) = info.mtime {
		table.set("mtime", mtime)?;
	}
	Ok(Value::Table(table))
}

/// Convert a `FileRecord` into a Lua table.
pub fn file_record_into_lua(record: FileRecord, lua: &Lua) -> ScriptResult<Value> {
	let table = lua.create_table()?;
	let info = record.info;
	table.set("path", info.path.as_str())?;
	table.set("name", info.name.as_str())?;
	table.set("stem", info.stem.as_str())?;
	table.set("ext", info.ext.as_str())?;
	if let Some(size) = info.size {
		table.set("size", size)?;
	}
	if let Some(ctime) = info.ctime {
		table.set("ctime", ctime)?;
	}
	if let Some(mtime) = info.mtime {
		table.set("mtime", mtime)?;
	}
	table.set("content", record.content.as_str())?;
	Ok(Value::Table(table))
}

/// Convert a `FileStats` into a Lua table.
pub fn file_stats_into_lua(stats: &FileStats, lua: &Lua) -> ScriptResult<Value> {
	let table = lua.create_table()?;
	table.set("number_of_files", stats.number_of_files)?;
	table.set("total_size", stats.total_size)?;
	if let Some(v) = stats.ctime_first {
		table.set("ctime_first", v)?;
	}
	if let Some(v) = stats.ctime_last {
		table.set("ctime_last", v)?;
	}
	if let Some(v) = stats.mtime_first {
		table.set("mtime_first", v)?;
	}
	if let Some(v) = stats.mtime_last {
		table.set("mtime_last", v)?;
	}
	Ok(Value::Table(table))
}

// endregion: --- Lua conversion helpers

// region:    --- Error helpers

/// Create an `AipApiError` with the given error code and message.
pub fn aip_file_error(code: impl Into<String>, message: &str) -> AipApiError {
	AipApiError::new(code, message.to_string())
}

// endregion: --- Error helpers
