use crate::{Error, Result};
use simple_fs::SPath;
use std::collections::HashSet;

pub const SPECIAL_DEFAULT_FOLDER_EXCLUDES: &[&str] = &[
	".aipack/",
	".git/",
	"target/",
	"node_modules/",
	".build/",
	"__pycache__/",
];

pub const SPECIAL_DEFAULT_FOLDER_EXCLUDE_GLOBS: &[&str] = &[
	"**/.aipack/**",
	"**/.git/**",
	"**/target/**",
	"**/node_modules/**",
	"**/.build/**",
	"**/__pycache__/**",
];

pub const GLOBS_TO_ALWAYS_EXCLUDES: &[&str] = &[
	"**/.DS_Store",
	".DS_Store",
	"**/Thumbs.db",
	"**/*.swp",
];

/// Resolves the effective exclude globs given include globs and custom exclude globs.
///
/// Explicitly matching a special folder in an include glob removes that folder
/// from the default exclusion set.
pub fn resolve_exclude_globs(
	include_globs: &[&str],
	custom_exclude_globs: &[&str],
) -> Result<Vec<String>> {
	let mut special_folder_excludes: HashSet<&'static str> =
		SPECIAL_DEFAULT_FOLDER_EXCLUDES.iter().copied().collect();

	for glob in include_globs {
		let glob = SPath::new(glob);
		let glob = glob.as_str().trim();

		let excludes_tmp: Vec<&'static str> = special_folder_excludes.iter().copied().collect();
		for exc in excludes_tmp {
			if glob.contains(exc) && special_folder_excludes.contains(exc) {
				special_folder_excludes.remove(exc);
			}
		}

		if glob.starts_with("../") || glob.starts_with("./..") {
			return Err(Error::custom(format!(
				"Glob '{glob}' starting with '../'.\nStarting glob with '../' is not supported at the moment."
			)));
		}
	}

	let mut resolved = Vec::new();

	if !special_folder_excludes.is_empty() {
		for (folder, glob) in SPECIAL_DEFAULT_FOLDER_EXCLUDES
			.iter()
			.zip(SPECIAL_DEFAULT_FOLDER_EXCLUDE_GLOBS.iter())
		{
			if special_folder_excludes.contains(*folder) {
				resolved.push((*glob).to_string());
			}
		}
	}

	for glob in GLOBS_TO_ALWAYS_EXCLUDES {
		resolved.push((*glob).to_string());
	}

	for glob in custom_exclude_globs {
		resolved.push((*glob).to_string());
	}

	Ok(resolved)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resolve_default_excludes() {
		let excludes = resolve_exclude_globs(&["**/*.rs"], &[]).unwrap();
		assert!(excludes.contains(&"**/target/**".to_string()));
		assert!(excludes.contains(&"**/node_modules/**".to_string()));
		assert!(excludes.contains(&"**/.git/**".to_string()));
		assert!(excludes.contains(&"**/.DS_Store".to_string()));
	}

	#[test]
	fn test_resolve_override_excludes() {
		let excludes = resolve_exclude_globs(&["target/**/*.rs"], &[]).unwrap();
		assert!(!excludes.contains(&"**/target/**".to_string()));
		assert!(excludes.contains(&"**/node_modules/**".to_string()));
		assert!(excludes.contains(&"**/.git/**".to_string()));
	}

	#[test]
	fn test_resolve_custom_excludes() {
		let custom = &["**/custom_ignore/**"];
		let excludes = resolve_exclude_globs(&["**/*.rs"], custom).unwrap();
		assert!(excludes.contains(&"**/custom_ignore/**".to_string()));
		assert!(excludes.contains(&"**/target/**".to_string()));
	}
}

// endregion: --- Tests
