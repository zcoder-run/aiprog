use super::AipRegistryResult;

use super::registry_types::{AipRegistryError, RegistrySelectionError};

pub(super) fn validate_path(path: &str) -> AipRegistryResult<()> {
	if path.is_empty() {
		return Err(AipRegistryError::InvalidPath("Path must not be empty".into()));
	}
	let segments: Vec<&str> = path.split('.').collect();
	if segments.len() < 2 {
		return Err(AipRegistryError::InvalidPath(format!(
			"Path '{}' must have at least one module/namespace segment and a function name segment",
			path
		)));
	}
	for seg in &segments {
		if seg.is_empty() {
			return Err(AipRegistryError::InvalidPath(format!(
				"Path '{}' contains empty segment(s)",
				path
			)));
		}
	}
	Ok(())
}

#[derive(Debug)]
pub(super) struct CompiledPathPattern {
	source: String,
	segments: Vec<PathPatternSegment>,
}

#[derive(Debug)]
enum PathPatternSegment {
	Literal(String),
	Single,
	Recursive,
}

impl CompiledPathPattern {
	pub fn source(&self) -> &str {
		&self.source
	}

	pub fn is_match(&self, path: &str) -> bool {
		let path_segments = path.split('.').collect::<Vec<_>>();
		matches_segments(&self.segments, &path_segments)
	}
}

pub(super) fn compile_path_patterns<I, S>(
	patterns: I,
) -> core::result::Result<Vec<CompiledPathPattern>, RegistrySelectionError>
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	patterns
		.into_iter()
		.map(|pattern| compile_path_pattern(pattern.as_ref()))
		.collect()
}

fn compile_path_pattern(
	pattern: &str,
) -> core::result::Result<CompiledPathPattern, RegistrySelectionError> {
	if pattern.is_empty() {
		return Err(invalid_pattern(pattern, "Pattern must not be empty"));
	}

	let segments = pattern
		.split('.')
		.map(|segment| match segment {
			"" => Err(invalid_pattern(pattern, "Pattern contains an empty segment")),
			"*" => Ok(PathPatternSegment::Single),
			"**" => Ok(PathPatternSegment::Recursive),
			literal if literal.contains('*') => Err(invalid_pattern(
				pattern,
				"Wildcard characters must occupy a complete segment",
			)),
			literal => Ok(PathPatternSegment::Literal(literal.to_string())),
		})
		.collect::<core::result::Result<Vec<_>, _>>()?;

	Ok(CompiledPathPattern {
		source: pattern.to_string(),
		segments,
	})
}

fn matches_segments(pattern: &[PathPatternSegment], path: &[&str]) -> bool {
	let Some((segment, remaining_pattern)) = pattern.split_first() else {
		return path.is_empty();
	};

	match segment {
		PathPatternSegment::Literal(literal) => {
			let Some((path_segment, remaining_path)) = path.split_first() else {
				return false;
			};
			literal == *path_segment && matches_segments(remaining_pattern, remaining_path)
		}
		PathPatternSegment::Single => {
			let Some((_, remaining_path)) = path.split_first() else {
				return false;
			};
			matches_segments(remaining_pattern, remaining_path)
		}
		PathPatternSegment::Recursive => {
			matches_segments(remaining_pattern, path)
				|| path
					.split_first()
					.is_some_and(|(_, remaining_path)| matches_segments(pattern, remaining_path))
		}
	}
}

fn invalid_pattern(pattern: &str, reason: &str) -> RegistrySelectionError {
	RegistrySelectionError::InvalidPattern {
		pattern: pattern.to_string(),
		reason: reason.to_string(),
	}
}
