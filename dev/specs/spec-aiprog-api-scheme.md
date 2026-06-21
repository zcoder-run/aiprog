# aip API Scheme

This document describes the idiomatic pattern used for Lua APIs under the `aip` namespace, as implemented in `aip.json` and `aip.web`. It serves as a reference for adding new modules or functions that follow the same conventions.

## Intent

Provide a consistent, self-documenting Lua API surface where:

- Every function lives under a `aip.<module>.<function>` path.
- Each function accepts a single table argument carrying typed parameters.
 Each function returns a Lua value appropriate for its result:
   - Simple results (e.g., a parsed JSON value, a string, a number) are returned directly as the native Lua type.
   - Structured results that carry metadata (e.g., HTTP response details) are returned as a Lua table with a `data` field and optional metadata.
- Errors are raised as structured Lua errors (or returned as part of the result) with an error code and message.
- TypeScript-style type definitions accompany the implementation so that autocompletion and documentation tools can leverage them.

## Code Design

### Namespace and Module Convention

All Lua APIs are grouped under the global `aip` table. Modules are sub-tables (e.g., `aip.json`, `aip.web`), and each function is a direct key inside its module table.

Examples:

- `aip.json.parse`
- `aip.json.stringify`
- `aip.web.get`
- `aip.web.post`

Constants belonging to a module are attached to the module table as regular keys (e.g., `aip.web.UA_AIPROG`). They are installed separately after the module table is created.

### Function Parameter Pattern

Every function expects a single argument, which is a Lua table. The table serves as a named-parameter mechanism and its structure is defined by a corresponding `...Params` type.

Naming convention:

- `Aip<Module><Function>Params` — e.g., `AipJsonParseParams`, `AipWebGetParams`.
- When the same set of parameters is needed by multiple functions, the Params type may be shared. For example, `AipJsonStringifyParams` is reused by both `stringify` and `stringify_pretty`.

The Params type exposes optional fields for optional behaviour, and required fields for mandatory inputs. All fields are available as top-level keys of the Lua table.

Example:

```typescript
interface AipJsonParseParams {
  /** The JSONC string to parse. Omit or set to nil to receive null. */
  text?: string;
}
```

In Rust, the Params struct is deserialized manually from the Lua table via the `AipFromLua` trait; the trait implementation maps table keys to Rust fields.


### Return Type and Data Wrapping

Each handler returns an `AipApiResult<T>` where `T` is the output type. The output type determines how the value is rendered in Lua.

#### Structured outputs (table with `data` field)

When additional metadata must accompany the result (e.g., HTTP status code, headers), the output type is a named struct with fields. The struct implements `AipIntoLua` (typically via `#[derive(AipIntoLua)]`, which uses serde to convert to a Lua table). The resulting Lua table always contains a `data` key holding the primary payload, along with any other defined fields.

Naming convention: `Aip<Module><Function>Output` or `Aip<Module>Output` when shared (e.g., `AipWebOutput`).

Example:

```rust
struct AipWebOutput {
    data: serde_json::Value,
    success: bool,
    status: u16,
    // ...
}
```

In Lua, the caller receives:

```lua
local res = aip.web.get({ url = "..." })
print(res.data, res.status)
```

#### Single‑value responses (raw value)

For functions whose natural result is a single value with no needed metadata, the output type is a newtype wrapper (single‑field tuple struct) around the value type. The wrapper implements `AipIntoLua` by delegating directly to the inner type's conversion; the Lua caller receives the value itself, not a table.

Naming convention: `Aip<Module><Function>Output`.

Example:

```rust
/// Output type for `aip.json.parse`.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipIntoLua, AipOutput)]
pub struct AipJsonParseOutput(pub serde_json::Value);
```

In Lua:

```lua
local val = aip.json.parse({ text = "..." })
-- val is the Lua representation of the JSON (table, string, …)
```

The `#[derive(AipIntoLua)]` macro detects single‑field tuple structs and generates a direct delegation `self.0.into_lua(lua)`, bypassing the serde round‑trip. For structs with multiple fields, the existing serde‑based table conversion is used.

> **Note**: Inner types must implement `AipIntoLua`. Standard library types (`serde_json::Value`, `String`, primitive numbers, `Vec<T>` where `T: AipIntoLua`) already do so.

### Error Handling


All handler errors are communicated through a unified `AipApiError` type. There is no per‑module or per‑function error variant; instead, the `code` field differentiates error categories.

```typescript
interface AipApiError {
  /** Machine‑readable error code, e.g. `"PARSE_FAILED"`, `"REQUEST_FAILED"`. */
  code: string;
  /** Human‑readable description of the error. */
  message: string;
  /** Optional additional context. */
  details?: string;
  /** Optional underlying cause. */
  cause?: string;
}
```

On the Lua side, this error is surfaced as a normal Lua error containing the `AipApiError` structure. Each function’s documentation **must** list the possible error codes it can return, so that Lua scripts can inspect and react to them.

Module authors are free to define their own error code strings. The codes should be concise, uppercase snake_case (e.g., `"PARSE_FAILED"`, `"STRINGIFY_FAILED"`, `"REQUEST_FAILED"`, `"CLIENT_BUILD_FAILED"`). A module‑level table of error codes may be exposed as a constant (e.g., `aip.json.ERROR_PARSE_FAILED`) if useful, but this is not required.

### TypeScript Type Definitions

The API types are described in TypeScript for conciseness. The definitions mirror the Rust structs, with `?` for optional fields and basic JSON types for `serde_json::Value`.

All types follow the naming schema:

```
Aip<Module><Function><Role>
```

Where `<Role>` is one of `Params`, `Output`, or `Error`.

Examples:

- `AipJsonParseParams`
- `AipJsonParseOutput`
- `AipJsonStringifyOutput`
- `AipWebGetParams`
- `AipWebOutput`

When a type is reused, the name may drop the function part (e.g., `AipWebOutput` instead of `AipWebGetOutput`). All output types use the `Output` suffix. This is clearly documented per function.

## Design Considerations

- **Single table argument**: Lua functions that accept many positional arguments can become hard to read. Using a single named-parameter table makes the API explicit and future-proof, as new optional fields can be added without breaking callers.
 **Return value shape**: Functions that produce a single value without additional metadata return that value directly, so callers can use it without unwrapping. Functions that need to provide metadata alongside the result use a table with a `data` field and additional fields, keeping the primary result accessible via `res.data`. This balances simplicity for common cases with clarity for more complex results.
- **Shared types**: When two functions have identical inputs or outputs, sharing the type reduces duplication and keeps the API consistent. The naming convention should still suggest the primary use or module.
 **Error representation**: The unified `AipApiError` type with string error codes provides a simple yet sufficient differentiation mechanism. Lua scripts can inspect the `code` field to handle specific errors. This avoids the complexity of multiple error types while remaining extensible through new code strings.
- **TypeScript documentation**: TypeScript interfaces provide a familiar, tool-friendly way to document the API shape without tying it to a particular language. They are used in the standard documentation (e.g., `doc-aip-json.md`).
