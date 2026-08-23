use crate::{AipRegistryBuilder, Result};

pub trait AipModule: Send + Sync + 'static {
	fn register(builder: AipRegistryBuilder) -> Result<AipRegistryBuilder>;
}
