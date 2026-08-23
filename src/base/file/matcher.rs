#![allow(unused)]

use regex::{Regex, RegexBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum ContentMatcher {
	Literal {
		text: String,
		ignore_case: bool,
	},
	Regex {
		regex: Regex,
		pattern: String,
		ignore_case: bool,
	},
}

impl ContentMatcher {
	pub fn new_literal(text: &str, ignore_case: bool) -> Self {
		let text = if ignore_case {
			text.to_lowercase()
		} else {
			text.to_string()
		};
		Self::Literal { text, ignore_case }
	}

	pub fn new_regex(pattern: &str, ignore_case: bool) -> Result<Self, regex::Error> {
		let regex = RegexBuilder::new(pattern).case_insensitive(ignore_case).build()?;

		Ok(Self::Regex {
			regex,
			pattern: pattern.to_string(),
			ignore_case,
		})
	}

	pub fn matches_file(&self, path: impl AsRef<Path>) -> std::io::Result<bool> {
		let file = File::open(path)?;
		let reader = BufReader::new(file);

		for line_res in reader.lines() {
			let line = match line_res {
				Ok(l) => l,
				Err(_) => continue,
			};

			if self.matches_str(&line) {
				return Ok(true);
			}
		}

		Ok(false)
	}

	pub fn matches_str(&self, content: &str) -> bool {
		match self {
			Self::Literal { text, ignore_case } => {
				if *ignore_case {
					content.to_lowercase().contains(text)
				} else {
					content.contains(text)
				}
			}
			Self::Regex { regex, .. } => regex.is_match(content),
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use tempfile::NamedTempFile;

	#[test]
	fn test_literal_case_sensitive() {
		let matcher = ContentMatcher::new_literal("Hello World", false);
		assert!(matcher.matches_str("Say Hello World to everyone"));
		assert!(!matcher.matches_str("Say hello world to everyone"));
	}

	#[test]
	fn test_literal_ignore_case() {
		let matcher = ContentMatcher::new_literal("Hello World", true);
		assert!(matcher.matches_str("Say Hello World to everyone"));
		assert!(matcher.matches_str("Say hello world to everyone"));
		assert!(!matcher.matches_str("Say hello there"));
	}

	#[test]
	fn test_regex_matching() {
		let matcher = ContentMatcher::new_regex(r"fn\s+test_[a-z0-9_]+", false).unwrap();
		assert!(matcher.matches_str("pub fn test_something() {}"));
		assert!(!matcher.matches_str("pub fn TEST_something() {}"));

		let ic_matcher = ContentMatcher::new_regex(r"fn\s+test_[a-z0-9_]+", true).unwrap();
		assert!(ic_matcher.matches_str("pub fn TEST_something() {}"));
	}

	#[test]
	fn test_matches_file_streaming() {
		let mut temp_file = NamedTempFile::new().unwrap();
		writeln!(temp_file, "line 1: header").unwrap();
		writeln!(temp_file, "line 2: target content here").unwrap();
		writeln!(temp_file, "line 3: footer").unwrap();

		let matcher = ContentMatcher::new_literal("target content", false);
		assert!(matcher.matches_file(temp_file.path()).unwrap());

		let non_matcher = ContentMatcher::new_literal("missing content", false);
		assert!(!non_matcher.matches_file(temp_file.path()).unwrap());
	}
}

// endregion: --- Tests
