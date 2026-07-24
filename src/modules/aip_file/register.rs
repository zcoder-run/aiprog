use crate::{AipRegistry, AipRegistryBuilder};
use crate::Result;

use super::file_read;

/// Build and return an [`AipRegistry`] containing all `aip.file` handlers.
///
/// This is the recommended way to obtain a registry for this module.
/// Use [`register`](register) if you need to add the handlers into an
/// existing registry.
#[allow(dead_code)]
pub fn init_registry() -> Result<AipRegistry> {
	Ok(register(AipRegistryBuilder::default())?.build())
}

pub fn register(registry: AipRegistryBuilder) -> Result<AipRegistryBuilder> {
	let registry = registry.merge(file_read::init_registry()?)?;

	Ok(registry)
}
