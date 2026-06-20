use crate::Result;
use crate::registry::AipRegistry;
use simple_fs::SPath;

use super::file_read;
use super::file_write;
use super::support::FileContext;

/// Build and return an [`AipRegistry`] containing all `aip.file` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
pub fn init_registry(file_ctx: Option<FileContext>) -> crate::Result<AipRegistry> {
	let mut registry = AipRegistry::from_empty();
	register(&mut registry, file_ctx)?;
	Ok(registry)
}

/// Register all `aip.file` handlers (read and write) into the given `AipRegistry`.
///
/// If no `FileContext` is provided, a default one using the current directory
/// will be created.
fn register(registry: &mut AipRegistry, file_ctx: Option<FileContext>) -> Result<()> {
	let ctx = match file_ctx {
		Some(ctx) => ctx,
		None => {
			let cwd = std::env::current_dir()
				.map_err(|e| crate::Error::cc("Failed to get current directory", e.to_string()))?;
			let spath = SPath::from_std_path_buf(cwd)
				.map_err(|e| crate::Error::cc("Failed to convert current_dir to SPath", e.to_string()))?;
			FileContext::new(spath)
		}
	};

	file_read::register_read(registry, ctx.clone())?;
	file_write::register_write(registry, ctx)?;

	Ok(())
}
