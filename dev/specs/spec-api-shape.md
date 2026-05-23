# AIProg API Shape Specification

This document defines the standard API shape for all capabilities exposed by AIProg to AI-authored Lua programs.
Following a single, consistent API pattern reduces prompt size, simplifies validation in the Rust runtime, and improves reliability for LLM code generation.

## Core Design Philosophy

- **Single Parameter:** Every API function accepts exactly one optional or required argument: a Lua table named `params`.
- **Consistent Response Structure:** Every API function returns a Lua table adhering to a standard response envelope, distinguishing between success (`result`) and error (`error`).
- **No Direct Nil Returns for Errors:** Errors do not return `nil, err_msg` (traditional Lua style). Instead, they return a standard error object in the envelope to allow unified handling.

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

All responses returned to the Lua script are structured Lua tables that follow a JSON-RPC-like success/error schema.

### TypeScript Definition

```ts
type ApiResponse<T> = ApiSuccessResponse<T> | ApiErrorResponse;

type ApiSuccessResponse<T> = {
  result: {
    data: T;
    [meta_key: string]: any; // Additional metadata like pagination limit, offset, etc.
  };
};

type ApiErrorResponse = {
  error: {
    message?: string; // Enum-style error code, UPPER_SNAKE_CASE
    data?: {
      full_message?: string; // Human-readable details
      cause?: string; // Underlying root cause if available
    };
  };
};
```

### Success Response Example

A successful execution returns a `result` property containing the actual returned `data` and any other optional metadata.

```lua
-- Lua table structure
{
  result = {
    data = {
      name = "AIProg"
    },
    mode = "flex" -- Example metadata
  }
}
```

```lua
-- Usage in Lua script
local response = aip.json.parse({ data = '{"name": "AIProg"}' })

if response.result then
  local data = response.result.data
  print(data.name)
end
```

### Error Response Example

An execution failure returns an `error` property containing structured error information.

```lua
-- Lua table structure
{
  error = {
    message = "INVALID_JSON",
    data = {
      full_message = "aip.json.parse failed. Expected double quote at line 1 column 2",
      cause = "EOF while parsing a string"
    }
  }
}
```

```lua
-- Usage in Lua script
local response = aip.json.parse({ data = '{"invalid' })

if response.error then
  print("Error message: " .. tostring(response.error.message))
  if response.error.data then
    print("Details: " .. tostring(response.error.data.full_message))
  end
end
```

## Guidelines for Developers

- **API Modules:** Ensure all functions check for a single input argument (a table).
- **TypeScript Docs:** When documenting functions for AI or human consumers, always provide the TypeScript type definitions for the input `Params` and the success `Result` data.
- **Backward Compatibility:** When migrating existing APIs, keep support for positional arguments internally but document only the single-params-table API in AI-facing docs.
