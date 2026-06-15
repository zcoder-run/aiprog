use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// region:    --- FileInfo

/// Metadata about a file, without content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileInfo {
	/// Resolved path (workspace-relative unless absolute was requested).
	pub path: String,
	/// File name with extension.
	pub name: String,
	/// File name without extension.
	pub stem: String,
	/// File extension, without the leading dot.
	pub ext: String,
	/// File size in bytes. Present when metadata is available.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub size: Option<u64>,
	/// Creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime: Option<i64>,
	/// Modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime: Option<i64>,
}

// endregion: --- FileInfo

// region:    --- FileRecord

/// Full file record, including content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileRecord {
	#[serde(flatten)]
	pub info: FileInfo,
	/// Full text content of the file.
	pub content: String,
}

// endregion: --- FileRecord

// region:    --- FileStats

/// Aggregate statistics over a set of files.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileStats {
	/// Number of files matched.
	pub number_of_files: usize,
	/// Sum of file sizes in bytes.
	pub total_size: u64,
	/// Earliest creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime_first: Option<i64>,
	/// Latest creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime_last: Option<i64>,
	/// Earliest modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime_first: Option<i64>,
	/// Latest modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime_last: Option<i64>,
}

// endregion: --- FileStats
