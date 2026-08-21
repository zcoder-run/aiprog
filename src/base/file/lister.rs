#![allow(unused)]

use super::matcher::ContentMatcher;
use simple_fs::{ListOptions, SMeta, SPath, list_files};

#[derive(Debug, Default)]
pub struct ListParams<'a> {
	pub globs: &'a [&'a str],
	pub exclude_globs: &'a [&'a str],
	pub content_matcher: Option<ContentMatcher>,
	pub with_meta: bool,
}

#[derive(Debug, Clone)]
pub struct MatchedFileEntry {
	pub path: SPath,
	pub meta: Option<SMeta>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileListStats {
	pub number_of_files: usize,
	pub total_size: u64,
	pub ctime_first: Option<i64>,
	pub ctime_last: Option<i64>,
	pub mtime_first: Option<i64>,
	pub mtime_last: Option<i64>,
}

impl FileListStats {
	pub fn record(&mut self, meta: &SMeta) {
		self.number_of_files += 1;
		self.total_size += meta.size;

		let ct = meta.created_epoch_us;
		self.ctime_first = Some(self.ctime_first.map_or(ct, |v| v.min(ct)));
		self.ctime_last = Some(self.ctime_last.map_or(ct, |v| v.max(ct)));

		let mt = meta.modified_epoch_us;
		self.mtime_first = Some(self.mtime_first.map_or(mt, |v| v.min(mt)));
		self.mtime_last = Some(self.mtime_last.map_or(mt, |v| v.max(mt)));
	}
}

pub fn list_matched_files(
	dir: &SPath,
	params: ListParams<'_>,
) -> crate::Result<Vec<MatchedFileEntry>> {
	let opts = ListOptions::default()
		.with_relative_glob()
		.with_exclude_globs(params.exclude_globs);

	let raw_files = list_files(dir, Some(params.globs), Some(opts))
		.map_err(|e| crate::Error::cc("File listing failed", e.to_string()))?;

	let mut matched_entries = Vec::new();

	for rel_path in raw_files {
		let full_path = dir.join(rel_path);

		if let Some(ref matcher) = params.content_matcher {
			let is_match = matcher
				.matches_file(full_path.as_str())
				.unwrap_or(false);
			if !is_match {
				continue;
			}
		}

		let meta = if params.with_meta {
			full_path.meta().ok()
		} else {
			None
		};

		matched_entries.push(MatchedFileEntry {
			path: full_path,
			meta,
		});
	}

	Ok(matched_entries)
}

pub fn compute_file_stats(
	dir: &SPath,
	params: ListParams<'_>,
) -> crate::Result<FileListStats> {
	let mut params = params;
	params.with_meta = true;

	let entries = list_matched_files(dir, params)?;
	let mut stats = FileListStats::default();

	for entry in entries {
		if let Some(meta) = entry.meta {
			stats.record(&meta);
		}
	}

	Ok(stats)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::{self, File};
	use std::io::Write;
	use tempfile::TempDir;

	#[test]
	fn test_list_matched_files_and_stats() {
		let temp_dir = TempDir::new().unwrap();
		let dir_path = SPath::from_std_path(temp_dir.path()).unwrap();

		let file1 = temp_dir.path().join("file1.txt");
		let mut f1 = File::create(&file1).unwrap();
		writeln!(f1, "apple orange banana").unwrap();

		let file2 = temp_dir.path().join("file2.txt");
		let mut f2 = File::create(&file2).unwrap();
		writeln!(f2, "grape strawberry melon").unwrap();

		let file3 = temp_dir.path().join("file3.rs");
		let mut f3 = File::create(&file3).unwrap();
		writeln!(f3, "fn apple_pie() {{}}").unwrap();

		let sub_dir = temp_dir.path().join("sub");
		fs::create_dir(&sub_dir).unwrap();
		let file4 = sub_dir.join("file4.txt");
		let mut f4 = File::create(&file4).unwrap();
		writeln!(f4, "apple juice").unwrap();

		// Test listing all txt files with content "apple"
		let matcher = ContentMatcher::new_literal("apple", false);
		let params = ListParams {
			globs: &["**/*.txt"],
			exclude_globs: &[],
			content_matcher: Some(matcher.clone()),
			with_meta: true,
		};

		let matched = list_matched_files(&dir_path, params).unwrap();
		assert_eq!(matched.len(), 2);

		// Compute stats
		let stats_params = ListParams {
			globs: &["**/*.txt"],
			exclude_globs: &[],
			content_matcher: Some(matcher),
			with_meta: true,
		};
		let stats = compute_file_stats(&dir_path, stats_params).unwrap();
		assert_eq!(stats.number_of_files, 2);
		assert!(stats.total_size > 0);
	}
}

// endregion: --- Tests
