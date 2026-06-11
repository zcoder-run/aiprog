use crate::registry::AipRegistry;
use crate::Result;
use simple_fs::SPath;

use super::file_read;
use super::support::FileContext;
use super::file_write;

/// Register all `aip.file` handlers (read and write) into the given `AipRegistry`.
///
/// If no `FileContext` is provided, a default one using the current directory
/// will be created.
pub fn register(registry: &mut AipRegistry, file_ctx: Option<FileContext>) -> Result<()> {
	let ctx = file_ctx.unwrap_or_else(|| {
		let cwd = SPath::from_std_path_buf(std::env::current_dir().unwrap())
			.expect("Failed to convert current_dir to SPath");
		FileContext::new(cwd)
	});

	file_read::register_read(registry, ctx.clone())?;
	file_write::register_write(registry, ctx)?;

	Ok(())
}
