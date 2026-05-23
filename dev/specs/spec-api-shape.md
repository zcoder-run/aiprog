# AIProg API Shape Specification

This document defines the standard API shape for all capabilities exposed by AIProg to AI-authored Lua programs.
Following a single, consistent API pattern reduces prompt size, simplifies validation in the Rust runtime, and improves reliability for LLM code generation.

## Core Design Philosophy

- **Single Parameter:** Every API function accepts exactly one optional or required argument: a Lua table named `params`.
 - **Consistent Response Structure:** Successful API functions return the result object directly at the root. Do not wrap success results in a `.result` field.
 - **No Direct Nil Returns for Errors:** Failures throw an error instead of returning `nil, err_msg`.

## Request Shape

All functions accept a single parameter table `params`.
If a function requires no arguments, `params` may be omitted or passed as an empty table `{}`.

### Lua Example

```lua
local response = aip.json.parse({
  data = '{"name": "AIProg"}',
  mode = "flex"
})
```

## Response Shape

 All responses returned to the Lua script are structured Lua tables that follow the success result schema. If the operation fails, a Lua error is thrown.

### Success Response Structure

Successful operations must return the result object (containing the `data` field) directly.

**CRITICAL:** Do NOT wrap the success result in a `.result` envelope.

## Params and Result Data Field Convention

All AI-facing API parameter and result types must use `data` for the primary payload field.

- `...Params` types should accept the main input payload as `.data`.
- `...Result` types should return the main output payload as `.data`.
- Do not use payload field names like `.content`, `.value`, or `.values` in the documented AI-facing API shape.
- Additional fields are allowed only for metadata, options, pagination, or other non-primary payload values.

### TypeScript Definition

```ts
type ApiSuccessResponse<T> = {
   data: T;
   [meta_key: string]: any; // Additional root-level metadata
};
```

 ### Success Response Example
 
 A successful execution returns the result object directly at the root.
 
 ```lua
 -- Usage in Lua script
 local result = aip.json.parse({ data = '{"name": "AIProg"}' })
 
 -- Access data directly from the returned object
 print(result.data.name)
 ```
 
 ### Error Handling
 
 An execution failure throws a Lua error. Scripts can use `pcall` if they need to catch and handle errors programmatically.
 
 ```lua
 -- Usage in Lua script
 local ok, res = pcall(aip.json.parse, { data = '{"invalid' })
 
 if not ok then
   -- res contains the error message
   print("Error: " .. tostring(res))
 end
 ```

## Guidelines for Developers

- **API Modules:** Ensure all functions check for a single input argument (a table).
- **TypeScript Docs:** When documenting functions for AI or human consumers, always provide the TypeScript type definitions for the input `Params` and the success `Result` data.
- **Params and Results:** Ensure the primary payload field is named `data` in both `...Params` and `...Result` types, rather than `content`, `value`, or `values`.
- **Backward Compatibility:** When migrating existing APIs, keep support for positional arguments internally but document only the single-params-table API in AI-facing docs.
