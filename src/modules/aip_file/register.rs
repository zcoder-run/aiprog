use crate::Result;
use crate::{AipRegistry, AipRegistryBuilder};

use super::FileModule;
use super::{file_read, file_write};

/// Build and return an [`AipRegistry`] containing all `aip.file` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
#[allow(dead_code)]
pub fn init_registry() -> Result<AipRegistry> {
	Ok(AipRegistryBuilder::default().add_module(FileModule)?.build())
}

pub fn register(registry: AipRegistryBuilder) -> Result<AipRegistryBuilder> {
	let registry = registry.merge(file_read::init_registry()?)?;
	let registry = registry.merge(file_write::init_registry()?)?;

	Ok(registry)
}
