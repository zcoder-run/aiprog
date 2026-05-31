# AipFn Specification

This document defines the `AipFn` pattern used to expose Rust functions to AI-authored Lua programs in AIProg.

The goal of `AipFn` is to make every AI-facing Lua API function consistent, typed, schema-friendly, and easy to register from Rust.

This specification describes the target architecture, an `rpc-router`-style design that splits responsibilities into three layers: a Lua-agnostic handler layer, a concrete registry layer, and a Lua adapter layer.

## Related specifications

- `dev/specs/spec-api-shape.md` defines the public AI-facing API shape.
- This document defines the Rust implementation pattern used to build functions that follow that shape.

## Core goals

- Every AI-facing function accepts exactly one optional or required Lua params table.
- Every successful function returns a Lua table with the result fields directly at the root.
- The primary payload field is always named `data`.
- Failures throw Lua errors instead of returning `nil, err`.
- Rust params, responses, and errors are strongly typed.
- Params and responses derive JSON schema metadata for documentation and tooling.
- Registration uses shared generic conversion logic instead of per-function Lua glue.
- The core handler abstraction is Lua-agnostic. `mlua::Lua`, `mlua::Value`, and Lua function signatures must not leak into the handler, registry, or normalized boundary layers.

## Public Lua API shape

Every function follows this shape:

```lua
local result = aip.module.function_name({
  data = "primary input payload"
})

print(result.data)
```

Successful responses return the response object directly:

```lua
{
  data = "primary output payload"
}
```

Do not wrap success values in a `result` field.

Errors are thrown as Lua errors:

```lua
local ok, res = pcall(aip.json.parse, {
  data = "{ invalid json"
})

if not ok then
  print(tostring(res))
end
```

This public Lua API shape is unchanged by the architecture described in this document. The shape remains a single params table, a root-level `data` field on success, and thrown Lua errors on failure, as defined in `dev/specs/spec-api-shape.md`.

## Architecture overview

The implementation is organized into three layers with a strict dependency direction.

```
Lua adapter layer        (depends on mlua)
        |
        v
Concrete registry layer  (no mlua)
        |
        v
Handler layer            (no mlua)
```

- The handler layer defines the generic typed handler abstraction and operates only on normalized `serde_json::Value` at its boundary.
- The registry layer stores concrete normalized handler representations and invokes them through normalized `serde_json::Value`.
- The Lua adapter layer is the only place that depends on `mlua`. It converts Lua params to normalized values, calls the registry, converts responses back to Lua, and converts handler errors to `mlua::Error`.

Normalized boundary values use `serde_json::Value`. The handler and registry layers never observe `mlua` types.

## Module layout

The handler architecture lives under:

- `src/script/support/handler/`

The handler module is organized as follows:

- `handler.rs`: the generic, Lua-agnostic handler trait, modeled on `rpc-router::Handler`. It defines how a typed handler is called with normalized params (`serde_json::Value`) and returns a normalized response result. It supports both sync and async handler kinds.

- `handler_params.rs`: the normalized params representation and conversion. It defines how an incoming normalized `serde_json::Value` is converted into a typed `Params` value via `DeserializeOwned`, including the empty-object special case, with no dependency on Lua.

- `handler_response.rs`: the normalized response handling. It defines how a typed response (`Serialize`) or a `serde_json::Value` is converted into a normalized `serde_json::Value`, preserving the root-level `data` response shape contract.

- `handler_error.rs`: the typed API errors and conversion into a normalized handler error. The primary error contract is conversion into the normalized handler error, not directly into `mlua::Error`. Lua conversion of errors happens only at the Lua adapter layer.

- `handler_wrapper.rs`: the type-erased wrappers for dynamic dispatch, modeled on `rpc-router::handler_wrapper`. A wrapper exposes a `call` that accepts normalized params (`serde_json::Value`) and returns a normalized response or normalized handler error. No `mlua` involvement.

- `impl_handlers.rs`: the macro-generated handler implementations for the supported function signatures (single params argument, sync and async).

- `registry.rs`: the Lua-agnostic registry that stores concrete handler metadata and type-erased handler wrappers (see the registry contract below).

- `lua_adapter.rs`: the single Lua boundary layer that bridges Lua and the registry (see the Lua adapter contract below).

- `mod.rs`: wires the handler submodules together and re-exports the public surface.

The legacy single-file implementation in `src/script/support/aip_fn_base.rs` is superseded by this module layout once the migration is complete.

## Handler layer

The handler layer defines the generic typed handler trait, modeled on `rpc-router::Handler`.

Key points:

- A handler is a plain Rust function or closure that takes a single typed `Params` argument and returns a typed `Result<Response, Error>`.
- The trait operates on normalized `serde_json::Value` at its public boundary. Typed conversion happens inside the handler implementation.
- Both sync and async handler kinds are supported. Async handlers return a pinned future of the normalized result.
- The handler layer has no dependency on `mlua`.

### Normalized params conversion

`handler_params.rs` converts an incoming normalized `serde_json::Value` into the typed params via `DeserializeOwned`.

Behavior:

- A normalized JSON object is deserialized directly into the params type.
- An empty normalized value (empty object) is treated as an empty JSON object, allowing APIs with only optional fields to accept an empty params table.
- Deserialization errors become a normalized handler error with code `INVALID_PARAMS`.

The empty-object special case is defined at the normalized-value level, independent of Lua. The Lua adapter is responsible for converting an empty Lua table to an empty normalized object before it reaches this layer.

### Normalized response conversion

`handler_response.rs` converts a typed response (`Serialize`) into a normalized `serde_json::Value`.

Because response structs already contain the `data` field, the normalized value has the expected root-level API shape. The Lua adapter converts this normalized value into a Lua table.

## Handler error layer

`handler_error.rs` defines the normalized handler error, modeled on `rpc-router::handler_error`.

- The normalized handler error can carry a typed application error through a type-erased holder, so the boundary code can downcast and inspect the original typed error when needed.
- An `IntoHandlerError` trait, with default implementations for common types, converts typed application errors into the normalized handler error.
- `AipApiError` remains the standard typed API error. Its primary contract is conversion into the normalized handler error.

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipApiError {
	pub code: String,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cause: Option<String>,
}
```

Fields:

- `code`: stable machine-readable error code, for example `PARSE_FAILED`.
- `message`: human-readable error message.
- `details`: optional additional information.
- `cause`: optional lower-level cause.

Conversion of a normalized handler error into `mlua::Error` is the responsibility of the Lua adapter layer, not the handler error type itself.

## Type-erased wrapper layer

`handler_wrapper.rs` provides the type-erased wrappers used to store handlers in the registry, modeled on `rpc-router::handler_wrapper`.

- A wrapper struct holds the concrete typed handler plus phantom marker types for its params, response, and error.
- A boxed trait object enables dynamic dispatch.
- The wrapper `call` accepts normalized params (`serde_json::Value`) and returns a normalized response (`serde_json::Value`) or a normalized handler error.
- The wrapper layer has no `mlua` involvement.

## Concrete registry layer

The registry stores concrete normalized handler representations.

Per-function, the registry stores:

- path or name (the Lua function name registered on the module table)
- sync or async kind
- params and response JSON schema metadata (`schemars`)
- a boxed type-erased handler wrapper

The registry contract:

- A builder-style append API registers a function under its name with its schema metadata and boxed wrapper.
- A call API invokes a handler by name with normalized `serde_json::Value` params and returns a normalized `serde_json::Value` response or a normalized handler error.
- The registry has no dependency on `mlua`.

## Lua adapter layer

`lua_adapter.rs` is the only place that depends on `mlua`.

The Lua adapter contract:

- Convert a Lua params value into a normalized `serde_json::Value`, including the empty-table case (an empty Lua table becomes an empty normalized object).
- Invoke the registry or handler wrapper with the normalized params.
- Convert the normalized `serde_json::Value` response back into a Lua value, preserving the root-level `data` shape.
- Convert a normalized handler error into `mlua::Error`, including the error code, message, optional details, and optional cause.

The adapter also provides a registration helper that installs registry functions onto a Lua module table, replacing the per-function `mlua` glue.

## Registration flow

A module builds a registry and installs it onto a Lua module table in `init_module`.

The conceptual flow:

1. Build a registry and append each typed function with its name, schema metadata, and handler.
2. Use the Lua adapter to install registry functions onto the Lua module table.
3. For each call, the Lua adapter:
   - converts the single Lua params argument to a normalized `serde_json::Value`,
   - invokes the registry handler by name,
   - converts the normalized response back to a Lua value,
   - converts any normalized handler error into a `mlua::Error`.

## Handler signature

A typed handler is a plain Rust function:

```rust
fn my_handler(params: MyParams) -> core::result::Result<MyResult, AipApiError> {
	Ok(MyResult {
		data: params.data,
	})
}
```

Async handlers return a future of the same typed result.

Handlers should contain the business logic.

Handlers should not:

- inspect raw Lua values
- manually create Lua tables
- manually serialize response envelopes
- return `nil, err`
- depend on `mlua` types

## Defining a new AipFn function

Use this checklist when adding a new AI-facing Lua function.

### 1. Define params

```rust
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipExampleEchoParams {
	pub data: String,
}
```

Use `data` for the primary input payload.

Use additional fields only for metadata, options, pagination, or other non-primary payload values.

### 2. Define response

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipExampleEchoResult {
	pub data: String,
}
```

Use `data` for the primary output payload.

### 3. Define handler

```rust
fn aip_example_echo_handler(params: AipExampleEchoParams) -> core::result::Result<AipExampleEchoResult, AipApiError> {
	Ok(AipExampleEchoResult {
		data: params.data,
	})
}
```

### 4. Register the function in the registry

Append the typed handler to the module registry with its Lua name and schema metadata, then install the registry onto the Lua module table through the Lua adapter.

## Current `aip.json` functions

The `aip.json` module uses the `AipFn` pattern for all functions.

File:

- `src/script/modules/aip_json.rs`

Functions:

- `aip.json.parse(params: { data?: string }) -> { data: any }`
- `aip.json.parse_ndjson(params: { data?: string }) -> { data: any[] }`
- `aip.json.stringify(params: { data: any }) -> { data: string }`
- `aip.json.stringify_pretty(params: { data: any }) -> { data: string }`

The module registers all functions through the registry and installs them with the Lua adapter.

Legacy positional argument support has been removed from this module. AI-facing code should use the single params-table shape only.

## Type and schema requirements

Params types should derive:

```rust
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
```

Response types should derive:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
```

Error types should implement:

- `schemars::JsonSchema`
- `IntoHandlerError`

`AipApiError` is the default error type unless a function needs a specialized error type.

## Naming conventions

Use names that match the Lua API path.

For a Lua function:

```lua
aip.json.stringify_pretty(...)
```

Use:

- params type: `AipJsonStringifyParams`
- result type: `AipJsonStringifyPrettyResult`
- handler: `aip_json_stringify_pretty_handler`

It is acceptable for related functions to share params types when the input shape is identical.

## Error code guidance

Use stable uppercase error codes.

Examples:

- `INVALID_PARAMS`
- `PARSE_FAILED`
- `STRINGIFY_FAILED`
- `IO_FAILED`
- `VALIDATION_FAILED`

Do not include dynamic details in the error code. Put dynamic details in `message`, `details`, or `cause`.

## Rust implementation guidelines

- Keep handler functions focused on business logic.
- Keep the handler, registry, and normalized boundary layers free of `mlua` references.
- Keep all Lua conversion isolated in the Lua adapter layer.
- Avoid `.unwrap()` and `.expect(...)`.
- Preserve root-level response shape, do not introduce a `result` wrapper.
- Use `data` as the primary payload field.
- Add schema derives to params and response types.
- Prefer a typed handler for every AI-facing function.
- Do not reintroduce legacy positional APIs for AI-facing functions unless explicitly required for backward compatibility.
