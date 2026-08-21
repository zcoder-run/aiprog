use crate::LuaExt as _;
use crate::Result;

/// The type of "extrude" to be performed.
/// - `Content`   Concatenate all lines outside of marked blocks into one string.
/// - `Fragments` (NOT SUPPORTED YET): Have a vector of strings for Before, In Between, and After
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub enum Extrude {
	Content,
}

#[allow(unused)]
impl Extrude {
	pub fn extract_from_table_value(value: &mlua::Table) -> Result<Option<Self>> {
		let extrude = value.x_get_string("extrude");
		extrude
			.map(|extrude| {
				if extrude == "content" {
					Ok(Extrude::Content)
				} else {
					Err(crate::Error::custom(
						"md_extract_blocks extrude must be = to 'content' for now",
					))
				}
			})
			.transpose()
	}
}
