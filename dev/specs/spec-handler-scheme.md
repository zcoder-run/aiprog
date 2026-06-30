# aip API Scheme

This document describes the idiomatic pattern used for Lua APIs under the `aip` namespace, as implemented in `aip.json` and `aip.web`. It serves as a reference for adding new modules or functions that follow the same conventions.

## Intent

Provide a consistent, self-documenting Lua API surface where:

- Every function lives under a `aip.<module>.<function>` path.
- Each function accepts a single table argument carrying typed parameters.
- Each function returns a Lua value appropriate for its result:
  - Simple results (e.g., a parsed JSON value, a string, a number) are returned directly as the native Lua type.
  - Structured results that carry metadata (e.g., HTTP response details) are returned as a Lua table with a `data` field and optional metadata.
- Errors are raised as standard Lua errors with a plain string message. There is no structured error code system.
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

 Each handler returns a `HandlerResult<T>`, which is a type alias for `core::result::Result<T, HandlerError>`. `HandlerError` is described in the Error Handling section. The output type `T` determines how the value is rendered in Lua.

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

The `AipOutput` derive macro is typically used alongside `AipIntoLua` to generate framework‑required trait implementations for output types.

> **Note**: Inner types must implement `AipIntoLua`. Standard library types (`serde_json::Value`, `String`, primitive numbers, `Vec<T>` where `T: AipIntoLua`) already do so.

### Handler Function Signature

A handler function is the Rust implementation of a Lua API function. It follows this signature:

```rust
fn handler_name(lua: &Lua, params: AipParams) -> HandlerResult<AipOutput> {
    // ... implementation
    Ok(AipOutput(...))
}
```

- `lua` is the `mlua::Lua` context.
- `params` is the deserialized parameters (extracted via `AipFromLua`). For functions without parameters, `()` is used.
- `AipOutput` is the output type (must implement `AipIntoLua` and typically uses `#[derive(AipOutput, AipIntoLua)]`).
- The function returns `HandlerResult<AipOutput>`. On success, it wraps the output in `Ok(...)`. On failure, it may return `Err(HandlerError::custom("...".into()))`.

The `AipOutput` derive macro provides the necessary boilerplate for the output type to integrate with the handler framework.

### Handler Registration

Every API function is registered into the global `AipRegistry` under a path such as `"json.parse"`. There are two equivalent ways to perform registration.

**Common parts** regardless of method:

- A params type `P` implementing `AipFromLua + JsonSchema + Send + Sync + 'static` (automatically satisfied by any type that derives `AipParams` and the necessary traits).
- An output type `O` implementing `AipIntoLua + JsonSchema + Send + Sync + 'static` (often via `#[derive(AipOutput, AipIntoLua)]`).
- Schemas for params, output, and `HandlerError` are computed automatically and stored in the registry entry.

**Option 1 — Manual registration with `register_sync` / `register_async`:**

The handler function (or closure) is passed directly to `register_sync` or `register_async`. This style gives full control and is useful for handwritten glue code.

```rust
// Example: register a sync handler
registry.register_sync::<MyParams, MyOutput, _>(
    "my_module.my_fn",
    MyHandler::new(),
)?;

// The handler impl (via AipSyncFnWrapper or AipAsyncFnWrapper) is provided by
// a blanket impl for closures that match the signature.
```

The `register_sync` call requires that the handler type implement `AipSyncFnWrapper<P, O>`; similarly `register_async` expects `AipAsyncFnWrapper<P, O>`. Typically you pass a closure or an object that implements the appropriate callable.

**Option 2 — Attribute‑based registration with `#[aip_handler]` and `register_handler`:**

The `#[aip_handler]` proc‑macro automatically generates a unit struct that implements the `AipHandler` trait, extracting metadata (title, description) from doc comments and constructing the registry entry. The handler itself is written as a plain Rust function with a single typed argument and `HandlerResult<O>` return.

```rust
/// # Parse JSON text
/// Parses a JSON string and returns the parsed value.
#[aip_handler]
fn parse(params: AipJsonParseParams) -> HandlerResult<AipJsonParseOutput> {
    // implementation
    Ok(AipJsonParseOutput(serde_json::Value::Null))
}

// Register it:
registry.register_handler("json.parse", parse)?;
```

The macro creates a hidden struct `__AiprogHandler_<fn_name>` that implements `AipHandler`. Calling `register_handler` consumes the function as a marker; the resulting `RegistryEntry` is identical in structure to one created by `register_sync`/`register_async`.

**Choosing a style:**

- Use `#[aip_handler]` and `register_handler` for the majority of cases — it provides automatic metadata extraction and reduces boilerplate.
- Use `register_sync`/`register_async` when you need to keep a stateful handler, share logic across paths, or implement custom lifecycle in the handler object.

Both styles produce the same registry schema and are fully interoperable.

### Error Handling


All handler errors use the `HandlerError` type, a simple string-based error. The Rust definition is an enum with a single `Custom(String)` variant (other variants may be added in the future). This keeps error handling lightweight and avoids introducing a complex error taxonomy.

```rust
// Simplified representation (actual Rust enum in handler_error.rs)
enum HandlerError {
    Custom(String),
}
```

On the Lua side, errors are raised as a standard Lua error with a plain string message. There is no structured error table or machine‑readable error code to inspect; the Lua script receives the error as a human‑readable string via the usual `pcall` mechanism.

Because there is no error code system, module‑specific error codes (like `"PARSE_FAILED"`) and module‑level error constant tables are not defined. When additional error information is needed, the string message should be descriptive enough to convey the context.

### TypeScript Type Definitions

The API types are described in TypeScript for conciseness. The definitions mirror the Rust structs, with `?` for optional fields and basic JSON types for `serde_json::Value`.

All types follow the naming schema:

```
Aip<Module><Function><Role>
```

Where `<Role>` is one of `Params` or `Output`.

Examples:

- `AipJsonParseParams`
- `AipJsonParseOutput`
- `AipJsonStringifyOutput`
- `AipWebGetParams`
- `AipWebOutput`

When a type is reused, the name may drop the function part (e.g., `AipWebOutput` instead of `AipWebGetOutput`). All output types use the `Output` suffix. This is clearly documented per function.

## Design Considerations

- **Single table argument**: Lua functions that accept many positional arguments can become hard to read. Using a single named-parameter table makes the API explicit and future-proof, as new optional fields can be added without breaking callers.
- **Return value shape**: Functions that produce a single value without additional metadata return that value directly, so callers can use it without unwrapping. Functions that need to provide metadata alongside the result use a table with a `data` field and additional fields, keeping the primary result accessible via `res.data`. This balances simplicity for common cases with clarity for more complex results.
- **Shared types**: When two functions have identical inputs or outputs, sharing the type reduces duplication and keeps the API consistent. The naming convention should still suggest the primary use or module.

- **Error representation**: Errors are raised as standard Lua errors with a descriptive string message. The Rust `HandlerError` type is a simple string-based error, keeping the API lightweight and avoiding a complex error taxonomy. Lua scripts can handle errors via `pcall` and inspect the error message string.

- **TypeScript documentation**: TypeScript interfaces provide a familiar, tool-friendly way to document the API shape without tying it to a particular language. They are used in the standard documentation (e.g., `doc-aip-json.md`).
