use crate::registry::AipRegistry;
use crate::Result;

// region:    --- Modules

mod aip_file;
mod aip_json;
mod aip_web;

// endregion: --- Modules

// region:    --- Combined Registry

/// Build and return a combined `AipRegistry` containing all built-in modules
/// (`aip.json`, `aip.web`, `aip.file`).
///
/// The `aip.file` module uses a default `FileContext` (current directory).
pub fn init_registry() -> crate::Result<AipRegistry> {
    let mut combined = AipRegistry::default();

    let json_registry = aip_json::init_registry()?;
    combined.merge(json_registry)?;

    let web_registry = aip_web::init_registry()?;
    combined.merge(web_registry)?;

    let file_registry = aip_file::register::init_registry(None)?;
    combined.merge(file_registry)?;

    Ok(combined)
}

// endregion: --- Combined Registry
