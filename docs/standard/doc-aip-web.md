# aip.web

- [`aip.web.get(params: AipWebGetParams)`](#aipwebgetparamsaipwebgetparams) — Performs an HTTP GET request.
- [`aip.web.post(params: AipWebPostParams)`](#aipwebpostparamsaipwebpostparams) — Performs an HTTP POST request.

## aip.web.get(params: AipWebGetParams)

Performs an HTTP GET request.

- **`data`** (string) — The URL to request.
- **`user_agent`** (optional, boolean or string) — Controls the User-Agent header.
- **`headers`** (optional, table) — Additional request headers.
- **`redirect_limit`** (optional, integer) — Maximum number of redirects to follow.
- **`parse`** (optional, boolean) — When `true` and the response Content-Type is JSON, the body is parsed into a Lua table.

Returns an [`AipWebResult`](#aipwebresult) table.

**Example:**

```lua
local res = aip.web.get({ data = "https://httpbin.org/json", parse = true })
-- res.data contains the parsed JSON object
```

## aip.web.post(params: AipWebPostParams)

Performs an HTTP POST request.

- **`data`** (string) — The URL to request.
- **`json`** (optional, any) — JSON value sent as the request body. Takes precedence over `body`.
- **`body`** (optional, string) — Raw string body (ignored when `json` is provided).
- **`user_agent`** (optional, boolean or string)
- **`headers`** (optional, table)
- **`redirect_limit`** (optional, integer)
- **`parse`** (optional, boolean)

Returns an [`AipWebResult`](#aipwebresult) table.

**Example:**

```lua
local res = aip.web.post({
    data = "https://httpbin.org/post",
    json = { key = "value" },
})
```

## Constants

- `aip.web.UA_AIPROG` : string `"aiprog"`
- `aip.web.UA_BROWSER`: string — a common Chrome user agent string.

## Common Types

### AipWebGetParams

Parameters for `aip.web.get`.

```typescript
interface AipWebGetParams {
  /** The URL to request. */
  data: string;
  /** User-Agent behavior. true → use "aiprog", false → no default UA, string → custom UA. */
  user_agent?: boolean | string;
  /** Extra request headers; values can be a string or an array of strings. */
  headers?: { [key: string]: string | string[] };
  /** Maximum number of redirects. */
  redirect_limit?: number;
  /** If true, parse JSON response when Content-Type is JSON. */
  parse?: boolean;
}
```

### AipWebPostParams

Parameters for `aip.web.post`.

```typescript
interface AipWebPostParams {
  data: string;
  /** JSON body; takes precedence over `body`. */
  json?: any;
  /** Raw string body. */
  body?: string;
  user_agent?: boolean | string;
  headers?: { [key: string]: string | string[] };
  redirect_limit?: number;
  parse?: boolean;
}
```

### AipWebResult

Return value for both `get` and `post`.

```typescript
interface AipWebResult {
  /** Response body: string, or parsed JSON object when `parse` was true and Content-Type is JSON. */
  data: any;
  /** True when status is 2xx. */
  success: boolean;
  /** HTTP status code. */
  status: number;
  /** Final URL after redirects. */
  url: string;
  /** Content-Type header, if present. */
  content_type?: string;
  /** Response headers (lower-case keys). Multiple values are joined with ", ". */
  headers: { [key: string]: string };
  /** Error description present only when `success` is false. */
  error?: string;
}
```

### AipWebUserAgent

```typescript
type AipWebUserAgent = boolean | string;
```

### AipWebHeaderValue

```typescript
type AipWebHeaderValue = string | string[];
```
