//! Write-related handlers for the `aip.file` module.
//!
//! Currently, write operations are not yet implemented. This file is a
//! placeholder so that the module structure is ready for future additions.
//!
//! When write functions are specified, they will follow the same aip API
//! scheme as the read functions.

use crate::Result;
use crate::registry::AipRegistry;

use super::support::FileContext;

/// Register write-related handlers (currently none).
///
/// When write handlers are added, this function will register them the same
/// way `file_read::register_read` does.
pub fn register_write(_registry: &mut AipRegistry, _ctx: FileContext) -> Result<()> {
	Ok(())
}
