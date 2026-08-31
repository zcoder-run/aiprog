# AIP Lua API Signature Patterns and Conventions

This specification defines the conventions and signature design patterns for all built-in Lua APIs exposed under the `aip.*` namespace.

## Core Design Principles

Built-in AIP Lua functions follow consistent patterns to remain ergonomic, predictable, and resilient across Lua scripts.

### 1. Single Table Argument for Parameters

All AIP functions accept parameters as a single Lua table with named fields rather than positional arguments.

```lua
-- Recommended
local parsed = aip.json.parse({ text = json_str })
local result = aip.web.get({ url = "https://example.com", parse = true })

-- Positional arguments are avoided except for legacy or scalar-only primitives
```

#### Advantages
- Self-documenting call sites.
- Extensibility without breaking signatures as new optional arguments are added.
- Clean interop with schema validation and tooling.

### 2. Nil-Friendly Input Handling

Functions that operate on values or text should gracefully handle `nil`, `null`, or omitted fields whenever sensible, returning `nil` or an empty representation rather than raising an execution error.

Examples:
- `aip.json.parse({})` or `aip.json.parse({ text = nil })` returns `nil`.
- `aip.json.parse_jsonl({})` returns `{}` (an empty array).
- `aip.json.stringify({})` returns `nil`.
- `aip.text.trim({ text = nil })` returns `nil`.
- `aip.text.format_size({ size = nil })` returns `nil`.

When a parameter is strictly required for the operation to make logical sense (such as `url` in `aip.web.get` or `path` in `aip.file.read`), omitting it raises a descriptive parameter error.

## Standard Property Naming Conventions

Across all modules, standard property names are reused for common concepts:

| Field Name | Type | Meaning / Usage |
|---|---|---|
| `text` | `string` | Text content to be parsed, transformed, or evaluated (e.g. `aip.json.parse`, `aip.text.trim`, `aip.time.parse`). |
| `data` | `any` | Structured Lua object, table, or value being passed in or returned (e.g. `aip.json.stringify`, `aip.web.get`). |
| `path` | `string` | Relative or target file/directory path (e.g. `aip.file.read`, `aip.file.exists`). |
| `base_dir` | `string` | Base directory used to resolve relative paths (e.g. `aip.file.*`). |
| `content` | `string` | File contents to write or returned from a read operation (e.g. `aip.file.write`, `aip.file.read`). |
| `globs` | `string \| string[]` | One or more glob patterns (e.g. `aip.file.list`). |
| `url` | `string` | Full HTTP/HTTPS target endpoint (e.g. `aip.web.get`, `aip.web.post`). |
| `headers` | `table` | Key-value pairs for HTTP headers (e.g. `aip.web.get`). |
| `query_params` | `table` | Key-value pairs appended as URL query parameters (e.g. `aip.web.get`). |
| `epoch_micro` | `integer` | Microseconds since Unix epoch (e.g. `aip.time.*`). |
| `utc_offset_seconds` | `integer` | UTC timezone offset in seconds (e.g. `aip.time.to_time_data`). |

## Return Value Patterns

Return types generally follow one of three patterns:

### 1. Direct Scalar or Unwrapped Value

When a function computes a single transformation or lookup, return the unnested value directly without wrapping it in an unnecessary table.

```lua
-- Returns boolean directly
local exists = aip.file.exists({ path = "config.json" })

-- Returns string directly (or nil if input is nil)
local trimmed = aip.text.trim({ text = "  hello  " })

-- Returns integer microsecond timestamp directly
local now_us = aip.time.now()
```

### 2. Direct Value or Structure from Parsers

Parsers return the decoded data directly (table, primitive, or array) or `nil` on absent input.

```lua
-- Returns the parsed JSON table/value directly
local data = aip.json.parse({ text = raw_json })

-- Returns an array of parsed records
local records = aip.json.parse_jsonl({ text = lines })
```

### 3. Structured Record with Metadata

When an operation produces multiple interrelated outputs or metadata alongside payload data, return a structured table.

```lua
-- File read returns both info metadata and raw content
local file = aip.file.read({ path = "doc.md" })
print(file.info.size, file.content)

-- Web requests return status, headers, and parsed/unparsed data
local res = aip.web.get({ url = "https://api.example.com", parse = true })
if res.success then
    print(res.status, res.data)
else
    print(res.error)
end
```

## Error Handling Conventions

AIP functions distinguish between fatal/invalid usage errors and expected domain outcomes:

- **Parameter Errors**: Passing invalid parameter shapes, wrong data types, or conflicting parameters raises a descriptive Lua error (halting execution unless caught with `pcall`).
- **Domain Errors / Rejections**: Operations that fail due to external conditions (like network failure or path policy violation) raise an error containing a distinct error message and prefix (e.g. `[PATH_POLICY_DENIED]`).
- **HTTP Status Codes**: `aip.web.*` operations succeed at the Lua level as long as the HTTP exchange completes, setting `success = false`, `status = <code >`, and `error = "<msg>"` on the returned table so scripts can inspect error responses.
