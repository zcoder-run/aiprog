pub mod file_read;
pub mod file_types;
pub mod file_write;
pub mod register;
pub mod support;

use crate::{AipModule, AipRegistryBuilder};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileModule;

impl AipModule for FileModule {
	fn register(builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		register::register(builder)
	}
}
