use super::AipRegistryResult;

use super::registry_types::AipRegistryError;

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
