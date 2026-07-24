use crate::{AipRegistryBuilder, Result};

pub trait AipModule: Send + Sync + 'static {
	fn register(&self, builder: AipRegistryBuilder) -> Result<AipRegistryBuilder>;
}
