//! Common support functions for the aip.file module.
//!
//! Contains path resolution, file listing using simple-fs, and shared
//! Lua conversion helpers.

use crate::base::file::ContentMatcher;
use crate::registry::{HandlerError, HandlerResult};
use mlua::{Lua, Value};
use simple_fs::{SPath, read_to_string};

use super::file_types::{DirContext, FileInfo, FileRecord, FileStats, ResolvedDirPath};

// region:    --- File listing

/// List files matching the given glob patterns.
///
/// Patterns starting with `!` are treated as exclusion patterns.
/// Returns the full, normalized `SPath` for each matched file.
pub fn list_files_matching(
	globs: &[String],
	base_dir: Option<&str>,
	matcher: Option<&ContentMatcher>,
	dir_context: &DirContext,
) -> crate::Result<Vec<ResolvedDirPath>> {
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

	let resolved_dir = dir_context
		.resolve_read(".", base_dir)
		.map_err(|e| crate::Error::cc("Directory policy rejected list path", e.to_string()))?;
	let dir = resolved_dir.path().clone();

	let entries = crate::base::file::list_matched_files(
		&dir,
		crate::base::file::ListParams {
			globs: &include_strs,
			exclude_globs: &exclude_strs,
			content_matcher: matcher.cloned(),
			with_meta: false,
		},
	)?;

	entries
		.into_iter()
		.map(|entry| {
			dir_context
				.authorize_existing_read(&entry.path)
				.map_err(|e| crate::Error::cc("Directory policy rejected listed path", e.to_string()))
		})
		.collect()
}

// endregion: --- File listing

// region:    --- File I/O helpers

/// Read the entire content of a file as a String.
pub fn read_file_content(path: &SPath) -> crate::Result<String> {
	read_to_string(path).map_err(|e| crate::Error::cc("Failed to read file", e.to_string()))
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
	let name = path.file_name().unwrap_or_default().to_string();
	let stem = path.file_stem().unwrap_or_default().to_string();
	let ext = path.extension().unwrap_or_default().to_string();

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
		path.diff(workspace_root).unwrap_or_else(|| path.clone()).as_str().to_string()
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
pub fn file_info_into_lua(info: FileInfo, lua: &Lua) -> crate::Result<Value> {
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
pub fn file_record_into_lua(record: FileRecord, lua: &Lua) -> crate::Result<Value> {
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
pub fn file_stats_into_lua(stats: &FileStats, lua: &Lua) -> crate::Result<Value> {
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

/// Create a `HandlerError::Custom` with the given error code and message.
pub fn aip_file_error(code: impl Into<String>, message: &str) -> HandlerError {
	HandlerError::custom(format!("[{}] {}", code.into(), message))
}

/// Validate that the given glob patterns are well-formed.
///
/// This function checks each pattern (including exclude patterns after
/// stripping the `!` prefix) using the `glob` crate. If any pattern is
/// invalid, a `HandlerError` with code `INVALID_GLOB` is returned.
pub fn validate_glob_patterns(globs: &[String]) -> HandlerResult<()> {
	for g in globs {
		let trimmed = g.trim();
		if trimmed.is_empty() {
			continue;
		}
		let pattern_str = if let Some(ex) = trimmed.strip_prefix('!') {
			ex
		} else {
			trimmed
		};
		if let Err(e) = glob::Pattern::new(pattern_str) {
			return Err(aip_file_error(
				"INVALID_GLOB",
				&format!("Invalid glob pattern: '{}': {}", pattern_str, e),
			));
		}
	}
	Ok(())
}

// endregion: --- Error helpers
