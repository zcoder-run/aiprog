# aip.json

The `aip.json` module provides functions to parse and serialize JSON content. By default, the `parse` function supports JSONC (JSON with comments and trailing commas). Parsing functions return `nil`/`null` when the input is `nil`.

- [`aip.json.parse(params: AipJsonParseParams)`](#aipjsonparseparamsaipjsonparseparams) — Parses a JSONC string into a Lua value.
- [`aip.json.parse_jsonl(params: AipJsonParseJsonlParams)`](#aipjsonparse_jsonlparamsaipjsonparsejsonlparams) — Parses an NDJSON (JSON Lines) string.
- [`aip.json.stringify(params: AipJsonStringifyParams)`](#aipjsonstringifyparamsaipjsonstringifyparams) — Serializes a value to a compact JSON string.
- [`aip.json.stringify_pretty(params: AipJsonStringifyParams)`](#aipjsonstringify_prettyparamsaipjsonstringifyparams) — Serializes a value to a pretty-printed JSON string.

## aip.json.parse(params: AipJsonParseParams)

Parses a JSONC string (JSON with optional comments and trailing commas) into a Lua value.

- **`text`** (optional, string) — The JSONC string to parse. When absent or `nil`, the result data is `null`.

Returns an [`AipJsonParseResult`](#aipjsonparseresult) table.

**Example:**

```lua
local res = aip.json.parse({ text = '{"name": "John", "age": 30}' })
-- res.data.name == "John"
```

## aip.json.parse_jsonl(params: AipJsonParseJsonlParams)

Parses an NDJSON (newline-delimited JSON) string. Empty lines are silently skipped.

- **`text`** (optional, string) — The NDJSON string to parse. When absent or `nil`, returns an empty list.

Returns an [`AipJsonParseJsonlResult`](#aipjsonparsejsonlresult) table.

**Example:**

```lua
local res = aip.json.parse_jsonl({ text = '{"name": "John"}\n{"name": "Jane"}' })
-- res.data is an array with two objects
```

## aip.json.stringify(params: AipJsonStringifyParams)

Serializes a Lua value (table, array, primitive) into a single-line JSON string.

- **`data`** (any) — The value to stringify.

Returns an [`AipJsonStringifyResult`](#aipjsonstringifyresult) table.

**Example:**

```lua
local res = aip.json.stringify({ data = { name = "John", age = 30 } })
-- res.text contains '{"name":"John","age":30}'
```

## aip.json.stringify_pretty(params: AipJsonStringifyParams)

Serializes a Lua value into a multi-line, indented JSON string.

- **`data`** (any) — The value to stringify.

Returns an [`AipJsonStringifyPrettyResult`](#aipjsonstringifyprettyresult) table.

**Example:**

```lua
local res = aip.json.stringify_pretty({ data = { name = "John", age = 30 } })
-- res.text contains a prettified JSON string with newlines and indentation
```

## Common Types

### AipJsonParseParams

```typescript
interface AipJsonParseParams {
  /** The JSONC string to parse. Omit or set to nil to get null. */
  text?: string;
}
```

### AipJsonParseResult

```typescript
interface AipJsonParseResult {
  /** The parsed value; can be any JSON type including null. */
  data: any;
}
```

### AipJsonParseJsonlParams

```typescript
interface AipJsonParseJsonlParams {
  /** The NDJSON string. Omit or set to nil to get an empty array. */
  text?: string;
}
```

### AipJsonParseJsonlResult

```typescript
interface AipJsonParseJsonlResult {
  /** Array of parsed values, one per line. */
  data: any[];
}
```

### AipJsonStringifyParams

Used by both `stringify` and `stringify_pretty`.

```typescript
interface AipJsonStringifyParams {
  /** The value to serialize. */
  data: any;
}
```

### AipJsonStringifyResult

```typescript
interface AipJsonStringifyResult {
  /** Compact JSON string. */
  text: string;
}
```

### AipJsonStringifyPrettyResult

```typescript
interface AipJsonStringifyPrettyResult {
  /** Pretty-printed JSON string. */
  text: string;
}
```
