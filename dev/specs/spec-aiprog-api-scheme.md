# aip API Scheme

This document describes the idiomatic pattern used for Lua APIs under the `aip` namespace, as implemented in `aip.json` and `aip.web`. It serves as a reference for adding new modules or functions that follow the same conventions.

## Intent

Provide a consistent, self-documenting Lua API surface where:

- Every function lives under a `aip.<module>.<function>` path.
- Each function accepts a single table argument carrying typed parameters.
- Each function returns a table with a mandatory `data` field and optional metadata.
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

Each function returns a single Lua table. The table always contains a `data` key holding the primary payload (the operation’s result). Additional metadata may be placed at the top level of the same table.

Naming convention:

- `Aip<Module><Function>Result` — e.g., `AipJsonParseResult`, `AipWebResult`.
- A Result type may be shared across multiple functions when the return shape is identical (e.g., `AipWebResult` for both `get` and `post`).

The `data` field can be any Lua value (nil, string, number, table). When the operation would logically return nothing, `data` is set to `nil` (Lua) or `null` (JSON). The Rust implementation uses `serde_json::Value` as the backing type and converts it to Lua via `AipIntoLua`.

Example:

```typescript
interface AipJsonParseResult {
  /** The parsed JSON value, or null for empty input. */
  data: any;
}

interface AipWebResult {
  /** Response body as string or parsed JSON object. */
  data: any;
  /** True for 2xx status codes. */
  success: boolean;
  /** HTTP status code. */
  status: number;
  /** Final URL after redirects. */
  url: string;
  /** Content-Type header, if present. */
  content_type?: string;
  /** Response headers (lower-case keys). */
  headers: { [key: string]: string };
  /** Error description when success is false. */
  error?: string;
}
```

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

Where `<Role>` is one of `Params`, `Result` (or `Error` in the future).

Examples:

- `AipJsonParseParams`
- `AipJsonParseResult`
- `AipWebGetParams`
- `AipWebResult`

When a type is reused, the name may drop the function part (e.g., `AipWebResult` instead of `AipWebGetResult`). This is acceptable as long as the reuse is clearly documented.

## Design Considerations

- **Single table argument**: Lua functions that accept many positional arguments can become hard to read. Using a single named-parameter table makes the API explicit and future-proof, as new optional fields can be added without breaking callers.
- **`data` as the primary payload**: By consistently wrapping the result payload in `data`, consumers can always access the core result with the same access pattern (`res.data`). Metadata fields reside at the same level, avoiding nesting.
- **Shared types**: When two functions have identical inputs or outputs, sharing the type reduces duplication and keeps the API consistent. The naming convention should still suggest the primary use or module.
 **Error representation**: The unified `AipApiError` type with string error codes provides a simple yet sufficient differentiation mechanism. Lua scripts can inspect the `code` field to handle specific errors. This avoids the complexity of multiple error types while remaining extensible through new code strings.
- **TypeScript documentation**: TypeScript interfaces provide a familiar, tool-friendly way to document the API shape without tying it to a particular language. They are used in the standard documentation (e.g., `doc-aip-json.md`).
