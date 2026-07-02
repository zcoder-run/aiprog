# aip.json

The `aip.json` module provides functions to parse and serialize JSON content. By default, the `parse` function supports JSONC (JSON with comments and trailing commas). When the input is `nil`, absent, or the null sentinel, `parse` and `stringify` return `nil`.

- [`aip.json.parse_jsonl(params: AipJsonParseJsonlParams)`](#aipjsonparse_jsonlparamsaipjsonparsejsonlparams) — Parses an NDJSON (JSON Lines) string.
- [`aip.json.stringify(params: AipJsonStringifyParams)`](#aipjsonstringifyparamsaipjsonstringifyparams) — Serializes a value to a compact JSON string.
- [`aip.json.stringify_pretty(params: AipJsonStringifyParams)`](#aipjsonstringify_prettyparamsaipjsonstringifyparams) — Serializes a value to a pretty-printed JSON string.

## aip.json.parse(params: AipJsonParseParams)

Parses a JSONC string (JSON with optional comments and trailing commas) into a Lua value.

- **`text`** (optional, string) — The JSONC string to parse. When absent or `nil`, the result is `nil`.

 Returns the parsed Lua value directly. The value can be any JSON type, including `nil` for JSON null.

 **Example:**

 ```lua
 local res = aip.json.parse({ text = '{"name": "John", "age": 30}' })
 -- res.name == "John"
 ```

## aip.json.parse_jsonl(params: AipJsonParseJsonlParams)

Parses an NDJSON (newline-delimited JSON) string. Empty lines are silently skipped.

- **`text`** (optional, string) — The NDJSON string to parse. When absent or `nil`, returns an empty list.

 Returns a Lua array (list) of parsed values directly.

 **Example:**

 ```lua
 local res = aip.json.parse_jsonl({ text = '{"name": "John"}\n{"name": "Jane"}' })
 -- res is an array with two objects
 ```

## aip.json.stringify(params: AipJsonStringifyParams)

Serializes a Lua value (table, array, primitive) into a single-line JSON string.

- **`data`** (any) — The value to stringify. When `data` is absent, `nil`, or the null sentinel, returns `nil`.

 Returns a compact JSON string, or `nil` if the data is missing or nil.

 **Example:**

 ```lua
 local res = aip.json.stringify({ data = { name = "John", age = 30 } })
 -- res contains '{"name":"John","age":30}'
 ```

## aip.json.stringify_pretty(params: AipJsonStringifyParams)

Serializes a Lua value into a multi-line, indented JSON string.

- **`data`** (any) — The value to stringify. When `data` is absent, `nil`, or the null sentinel, returns `nil`.

 Returns a pretty-printed JSON string, or `nil` if the data is missing or nil.

 **Example:**

 ```lua
 local res = aip.json.stringify_pretty({ data = { name = "John", age = 30 } })
 -- res contains a prettified JSON string with newlines and indentation
 ```

## Common Types

### AipJsonParseParams

```typescript
interface AipJsonParseParams {
  /** The JSONC string to parse. Omit or set to nil to get null. */
  text?: string;
}
```

### AipJsonParseResponse
### AipJsonParseOutput

The parsed JSON value returned directly to Lua.

```typescript
type AipJsonParseOutput = any;
```

### AipJsonParseJsonlParams

```typescript
interface AipJsonParseJsonlParams {
  /** The NDJSON string. Omit or set to nil to get an empty array. */
  text?: string;
}
```

### AipJsonParseJsonlOutput

The array of parsed values returned directly to Lua.

```typescript
type AipJsonParseJsonlOutput = any[];
```

### AipJsonStringifyParams

Used by both `stringify` and `stringify_pretty`.

```typescript
interface AipJsonStringifyParams {
  /** The value to serialize. */
  data: any;
}
```

### AipJsonStringifyOutput

The compact JSON string returned directly to Lua.

```typescript
type AipJsonStringifyOutput = string;
```

### AipJsonStringifyPrettyOutput

The pretty-printed JSON string returned directly to Lua.

```typescript
type AipJsonStringifyPrettyOutput = string;
```
