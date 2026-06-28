use crate::{AipFromLua, AipIntoLua};
use schemars::JsonSchema;

// region:    --- AipParams

/// Unified trait for handler params types.
///
/// Any type used as handler params must be deserializable from a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipParams: AipFromLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipParams

// region:    --- AipOutput

/// Unified trait for handler output types.
///
/// Any type used as a handler output must be serializable to a Lua value, have a
/// JSON schema, and be thread-safe. The blanket implementation ensures any
/// type satisfying the component bounds automatically qualifies.
pub trait AipOutput: AipIntoLua + JsonSchema + Send + Sync + 'static {}

// endregion: --- AipOutput

// region:    --- AipHandlerMeta

/// Metadata extracted from handler doc comments.
///
/// Carries the optional title (first ATX heading) and description
/// (remaining doc lines). The `#[aip_handler]` proc-macro populates
/// this via a generated `__aiprog_meta_<ident>()` helper.
pub struct AipHandlerMeta {
    pub(crate) description: Option<String>,
    pub(crate) title: Option<String>,
}

// endregion: --- AipHandlerMeta
