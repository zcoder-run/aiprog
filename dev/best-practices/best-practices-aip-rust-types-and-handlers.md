# Best Practices for AIP Rust Types and Handlers

This document captures patterns and conventions for defining Params and Output types in the AIP (AI Program) system Lua handler interface.

## `#[serde_with::skip_serializing_none]` on container types

Always apply `#[serde_with::skip_serializing_none]` on any Params or Output struct that contains at least one `Option` field.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct MyParams {
    pub required_field: String,
    pub optional_field: Option<String>,
}
```

This macro automatically skips serialization of `None` values for all `Option` fields in the struct, keeping the serialized output clean (no `"optional_field": null` entries).

## Do not use `#[serde(default)]` on `Option` fields

`Option` fields default to `None` during deserialization when the key is missing from the input. Adding `#[serde(default)]` is redundant and should be omitted.

Incorrect:

```rust
pub struct MyParams {
    #[serde(default)]
    pub text: Option<String>,
}
```

Correct:

```rust
pub struct MyParams {
    pub text: Option<String>,
}
```

## Remove manual `#[serde(skip_serializing_if = "Option::is_none")]`

When `#[serde_with::skip_serializing_none]` is applied at the container level, remove any manual `#[serde(skip_serializing_if = "Option::is_none")]` annotations from individual fields. The container attribute handles this uniformly.

Incorrect (redundant when container attribute is present):

```rust
#[serde_with::skip_serializing_none]
pub struct MyOutput {
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

Correct:

```rust
#[serde_with::skip_serializing_none]
pub struct MyOutput {
    pub data: String,
    pub error: Option<String>,
}
```

## Dependency setup

Ensure `serde_with` is in `Cargo.toml` with the `macros` feature enabled:

```toml
[dependencies]
serde_with = { version = "3", features = ["macros"] }
```

The `use serde_with::skip_serializing_none;` import is not required; the macro can be used directly with the `#[serde_with::skip_serializing_none]` qualified path, which is the preferred style in AIP code.

## Type categories

### Params types

- Derive `Debug, Clone, serde::Deserialize, schemars::JsonSchema`.
- Implement `AipFromLua` manually (hand-written deserialization from Lua values).
- Implement `AipParams` (marker trait).
- Do **not** derive `serde::Serialize` unless the type is also used as an output.

### Output types

- Derive `Debug, Clone, serde::Serialize, schemars::JsonSchema`.
- Implement `AipIntoLua` to convert the Rust struct into a Lua value.
- Implement `AipOutput` (marker trait).

## Example: Complete Params type

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

## Example: Complete Output type

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
