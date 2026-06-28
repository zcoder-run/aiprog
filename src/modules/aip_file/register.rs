use crate::AipRegistry;
use crate::{Error, Result};
use simple_fs::SPath;

use super::file_read;
use super::support::FileContext;

/// Build and return an [`AipRegistry`] containing all `aip.file` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
pub fn init_registry(file_ctx: Option<FileContext>) -> Result<AipRegistry> {
	// -- setup file context (need to change this, not used yet)
	// NOTE: this FileContext scheme will change completely (not good as it is, too 'static')
	let ctx = match file_ctx {
		Some(ctx) => ctx,
		None => {
			let cwd =
				std::env::current_dir().map_err(|e| Error::cc("Failed to get current directory", e.to_string()))?;
			let spath = SPath::from_std_path_buf(cwd)
				.map_err(|e| Error::cc("Failed to convert current_dir to SPath", e.to_string()))?;
			FileContext::new(spath)
		}
	};

	// --
	let mut registry = AipRegistry::from_empty();
	registry.merge(file_read::init_registry_with_ctx(ctx)?)?;

	Ok(registry)
}
