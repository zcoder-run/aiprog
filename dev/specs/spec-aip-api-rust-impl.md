# AIP Rust Module Implementation Patterns

This document defines the architecture, code layout, and implementation conventions for Rust modules providing Lua functions in the AIP framework.

## Module Directory and File Structure

Each AIP module is placed under `src/modules/` and follows one of two layouts:

### Single-File Module (`src/modules/aip_<module>.rs` + `src/modules/aip_<module>_tests.rs`)

Suitable for small or self-contained modules (e.g. `aip_json.rs`, `aip_text.rs`, `aip_time.rs`, `aip_web.rs`).

```text
src/modules/
├── aip_text.rs         # Module definition, params, outputs, handlers
├── aip_text_tests.rs   # Comprehensive async/sync Lua integration tests
```

### Multi-File Submodule (`src/modules/aip_<module>/`)

Suitable for larger domains with multiple related sub-domains (e.g. `aip_file/`).

```text
src/modules/aip_file/
├── mod.rs              # Module struct, exports, registration
├── file_types.rs       # Shared structs, enums, policy types
├── file_read.rs        # Read/list handlers and params
├── file_read_tests.rs  # Read handler unit/integration tests
├── file_write.rs       # Write/copy/delete handlers and params
├── file_write_tests.rs # Write handler tests
├── register.rs         # Submodule registry assembly
└── support.rs          # Shared filesystem utility functions
```

## Internal Module Layout

Each module source file is organized using standard region comments:

```rust
#![allow(non_camel_case_types)]

use crate::derive::{AipOutput, AipParams};
use crate::registry::{HandlerError, HandlerResult};
use crate::{AipFromLua, AipIntoLua, AipModule, AipRegistryBuilder, HandlerCallContext};
use aiprog_macros::{aip_handler, register_handler};

// region:    --- Module

#[derive(Debug, Clone, Copy, Default)]
pub struct SampleModule;

impl AipModule for SampleModule {
    fn register(builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
        register(builder)
    }
}

pub fn register(mut registry: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
    register_handler!(registry, "aip.sample.action", aip_sample_action_handler)?;
    Ok(registry)
}

// endregion: --- Module

// region:    --- Types (Params & Outputs)

// Parameter structs and Output wrappers

// endregion: --- Types

// region:    --- Lua Traits

// Manual or customized AipFromLua / AipIntoLua implementations

// endregion: --- Lua Traits

// region:    --- Handlers

// #[aip_handler] functions implementing module logic

// endregion: --- Handlers

// region:    --- Support

// Internal helper functions

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "aip_sample_tests.rs"]
mod tests;

// endregion: --- Tests
```

## Parameter Types (`AipParams`)

Each handler accepts a typed parameter struct deserialized from Lua.

### Conventions
- Derive `Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema`.
- Apply `#[serde_with::skip_serializing_none]` when the struct contains `Option` fields.
- For types with custom Lua conversion rules, implement `AipFromLua` explicitly using `LuaExt` helper methods (`x_try_get_string`, `x_try_get_bool`, `x_try_get_value`, `x_as_lua_str`, etc.).
- Implement `AipParams` (or derive it via `#[derive(AipParams)]`).
- Include doc comments on all fields for automatic schema documentation.

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
#[serde_with::skip_serializing_none]
pub struct AipSampleActionParams {
    /// The target text to process.
    pub text: Option<String>,

    /// Formatting option flag.
    pub pretty: Option<bool>,
}
```

## Output Types (`AipOutput`)

Return values are modeled with explicit output types implementing `AipOutput` and `AipIntoLua`.

### 1. Newtype Wrapper for Scalars and Direct Values

For single-value outputs (e.g. primitives, strings, options, direct JSON values), use a single-field tuple struct:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipSampleOutput(pub Option<String>);

impl AipIntoLua for AipSampleOutput {
    fn into_lua(self, lua: &mlua::Lua) -> crate::Result<mlua::Value> {
        match self.0 {
            Some(s) => s.into_lua(lua),
            None => Ok(mlua::Value::Nil),
        }
    }
}

impl AipOutput for AipSampleOutput {}
```

Alternatively, derive `AipIntoLua` and `AipOutput` when standard delegation applies:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipOutput)]
pub struct AipJsonParseOutput(pub Option<serde_json::Value>);
```

### 2. Structured Table Output

For structs representing multi-field Lua tables:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipRecordOutput {
    pub name: String,
    pub count: usize,
    pub meta: Option<String>,
}

impl AipIntoLua for AipRecordOutput {
    fn into_lua(self, lua: &mlua::Lua) -> crate::Result<mlua::Value> {
        let table = lua.create_table()?;
        table.set("name", self.name)?;
        table.set("count", self.count)?;
        if let Some(meta) = self.meta {
            table.set("meta", meta)?;
        }
        Ok(mlua::Value::Table(table))
    }
}

impl AipOutput for AipRecordOutput {}
```

## Handler Implementation

Handler functions are declared with the `#[aip_handler]` attribute macro.

```rust
/// Processes input text and returns formatted output.
#[aip_handler]
fn aip_sample_action_handler(
    call_ctx: HandlerCallContext,
    params: AipSampleActionParams,
) -> HandlerResult<AipSampleOutput> {
    let Some(text) = params.text else {
        return Ok(AipSampleOutput(None));
    };

    let processed = text.trim().to_uppercase();
    Ok(AipSampleOutput(Some(processed)))
}
```

### Context Access
If the handler requires runtime context (such as `DirContext` in file handlers), use `call_ctx.with`:

```rust
let resolved = call_ctx
    .with::<DirContext, _>(|dir| dir.resolve_read(&params.path, params.base_dir.as_deref()))?
    .map_err(|e| HandlerError::custom(format!("[PATH_POLICY_DENIED] {e}")))?;
```

### Error Reporting
Errors are raised via `HandlerError::custom("message")` or `HandlerError::custom_from_err(err)`. Keep error messages actionable and concise.

## Integration Testing Pattern

Test files (`aip_<module>_tests.rs`) follow a standardized async test structure using `_test_support`:

```rust
type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

use crate::_test_support::{eval_script, setup_lua_engine};
use crate::modules::SampleModule;
use crate::AipRegistryBuilder;
use serde_json::json;

fn setup_sample_engine() -> crate::Result<crate::ScriptEngine> {
    setup_lua_engine(|| Ok(AipRegistryBuilder::default().add_module(SampleModule)?.build()))
}

#[tokio::test]
async fn test_aip_sample_action_basic() -> Result<()> {
    // -- Setup & Fixtures
    let engine = setup_sample_engine()?;
    let script = r#"
        return aip.sample.action({ text = "  hello world  " })
    "#;

    // -- Exec
    let res = eval_script(&engine, script).await?;

    // -- Check
    assert_eq!(res, json!("HELLO WORLD"));
    Ok(())
}
```
