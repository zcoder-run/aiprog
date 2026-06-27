# Best Practices for AIP Rust Types and Handlers

This document describes the idiomatic patterns for creating Lua handler functions and their associated types in the AIP system.

## Handler Function Signature

A handler function implements a Lua API function. It follows a standard signature:

```rust
fn handler_name(lua: &Lua, params: ParamsType) -> HandlerResult<OutputType> {
    // implementation
    Ok(output)
}
```

- `lua` is the `mlua::Lua` context.
- `params` is the deserialized parameters, extracted automatically via `AipFromLua`. For functions that take no parameters, use `()`.
- `OutputType` must implement `AipIntoLua` (and usually `AipOutput`).
- The return type is `HandlerResult<OutputType>`, which on success wraps the output in `Ok(...)`, and on failure returns `Err(HandlerError::custom("...".into()))`.

## Params Types

Each handler function accepts a single Lua table argument. The structure of that table is defined by a Params struct.

- Derive `Debug, Clone, serde::Deserialize, schemars::JsonSchema`.
- Implement `AipFromLua` manually (hand-written deserialization from the Lua table).
- Implement the `AipParams` marker trait.
- Do **not** derive `serde::Serialize` unless the type is also used as an output.

Apply `#[serde_with::skip_serializing_none]` on the container when the struct has at least one `Option` field. This keeps serialized outputs clean by omitting `None` values.

```rust
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipWebGetParams {
    pub url: String,
    pub user_agent: Option<AipWebUserAgent>,
    pub headers: Option<HashMap<String, AipWebHeaderValue>>,
    pub redirect_limit: Option<usize>,
    pub parse: Option<bool>,
}
```

`Option` fields automatically default to `None` when the corresponding key is absent from the Lua table, so no additional attribute is needed.

### Params as function documentation

The Params struct, including its field names and doc comments, serves as the primary description of the Lua function’s interface. Use doc comments on each field to explain its purpose, constraints, and default behaviour.

### Reusing Params types

When several functions share the same parameter shape, reuse a single Params type instead of duplicating it. For example, `AipJsonStringifyParams` is used by both `stringify` and `stringify_pretty`.

## Output Types

The output type determines how the result is rendered in Lua. There are two patterns:

### Single‑value responses

When the function returns a single value with no additional metadata, define a newtype wrapper (a single‑field tuple struct) that implements `AipIntoLua` by delegating to the inner type’s conversion. The Lua caller receives the value directly, not a table.

```rust
/// Output type for `aip.json.parse`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipOutput)]
pub struct AipJsonParseOutput(pub serde_json::Value);
```

The `#[derive(AipIntoLua)]` macro on a single‑field tuple struct generates a direct delegation to the inner type’s `into_lua`.

### Structured outputs (table with `data` field)

When the result needs to carry metadata (e.g., HTTP status code, headers), use a named struct with a `data` field and any additional fields. The struct implements `AipIntoLua` (via serde‑based table conversion). The resulting Lua table always contains a `data` key plus the other defined fields.

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipWebOutput {
    pub data: serde_json::Value,
    pub success: bool,
    pub status: u16,
    pub url: String,
    pub content_type: Option<String>,
    pub headers: HashMap<String, String>,
    pub error: Option<String>,
}
```

- Derive `Debug, Clone, serde::Serialize, schemars::JsonSchema`.
- Implement `AipIntoLua` (via derive macros) and the `AipOutput` marker trait.
- Use `#[serde_with::skip_serializing_none]` to omit `None` fields from the Lua table.

### Reusing Output types

When multiple functions produce the same output shape, define a shared output type at the module level (e.g., `AipWebOutput`) rather than per‑function types.

## Sharing Types Among Functions of the Same Category

Within a module or a set of related functions, prefer sharing Params and Output types when the interfaces are identical. This reduces duplication, keeps the API consistent, and simplifies documentation.

- Use module‑level type names: `AipWebOutput`, `AipJsonStringifyParams`, etc.
- Only create a per‑function type when the parameters or output shape genuinely differ.

## Error Handling

Handler errors use the `HandlerError::custom("message")` constructor. Errors are raised as standard Lua errors with the provided string message. Keep error messages descriptive so that Lua callers can understand the failure.

```rust
Err(HandlerError::custom("Failed to parse JSON: trailing comma at line 3"))
```

There is no structured error code system; the string message is the primary diagnostic.

## Dependency Setup

Add `serde_with` to `Cargo.toml` with the `macros` feature enabled:

```toml
[dependencies]
serde_with = { version = "3", features = ["macros"] }
```

The `#[serde_with::skip_serializing_none]` attribute is used directly; importing the macro is not required.
