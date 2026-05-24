# AipFn Specification

This document defines the `AipFn` pattern used to expose Rust functions to AI-authored Lua programs in AIProg.

The goal of `AipFn` is to make every AI-facing Lua API function consistent, typed, schema-friendly, and easy to register from Rust.

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

## Rust support module

The core implementation lives in:

- `src/script/support/aip_fn_base.rs`

The support module defines:

- `AipApiError`
- `IntoAipLuaError`
- `AipFromLua`
- `AipToLua`
- `AipFn`
- `register_aip_fn`
- `lua_params_from_value`
- `return_success_envelope`
- `return_error_envelope`

## `AipApiError`

`AipApiError` is the standard typed error for AI-facing APIs.

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

## `IntoAipLuaError`

`IntoAipLuaError` converts typed API errors into `mlua::Error`.

```rust
pub trait IntoAipLuaError {
	fn into_aip_lua_error(self) -> mlua::Error;
}
```

`AipApiError` implements this trait by converting the structured error into `mlua::Error::RuntimeError`.

The formatted error includes:

- error code
- message
- optional details
- optional cause

## Lua conversion traits

`AipFn` uses local conversion traits rather than requiring every params or response type to implement `mlua::FromLua` or `mlua::IntoLua` directly.

### `AipFromLua`

```rust
pub trait AipFromLua: DeserializeOwned {
	fn aip_from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self>
	where
		Self: Sized,
	{
		lua_params_from_value(value)
	}
}
```

There is a blanket implementation for all `DeserializeOwned` types.

This means params structs usually only need:

```rust
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct MyParams {
	pub data: String,
}
```

### `AipToLua`

```rust
pub trait AipToLua: Serialize {
	fn aip_to_lua(self, lua: &Lua) -> mlua::Result<Value>
	where
		Self: Sized,
	{
		return_success_envelope(lua, self)
	}
}
```

There is a blanket implementation for all `Serialize` types.

This means response structs usually only need:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct MyResult {
	pub data: String,
}
```

## `AipFn` trait

Each Lua function is represented by a marker struct that implements `AipFn`.

```rust
pub trait AipFn {
	const NAME: &'static str;

	type Params: AipFromLua + JsonSchema;
	type Response: AipToLua + JsonSchema;
	type Error: JsonSchema + IntoAipLuaError;

	fn register_typed<H>(lua: &Lua, table: &mlua::Table, handler: H) -> mlua::Result<()>
	where
		H: Fn(Self::Params) -> Result<Self::Response, Self::Error> + 'static,
		Self: Sized,
	{
		register_aip_fn::<Self, H>(lua, table, handler)
	}
}
```

Associated items:

- `NAME`: Lua function name registered on the module table.
- `Params`: typed params table.
- `Response`: typed success response.
- `Error`: typed error.

## Handler signature

A typed handler is a plain Rust function:

```rust
fn my_handler(params: MyParams) -> core::result::Result<MyResult, AipApiError> {
	Ok(MyResult {
		data: params.data,
	})
}
```

Handlers should contain the business logic.

Handlers should not:

- inspect raw Lua values
- manually create Lua tables
- manually serialize response envelopes
- return `nil, err`
- call `mlua` conversion helpers directly unless the function has a special conversion need

## Registration flow

A module registers functions in `init_module`.

```rust
pub fn init_module(lua: &Lua) -> Result<Table> {
	let table = lua.create_table()?;

	MyFn::register_typed(lua, &table, my_handler)?;

	Ok(table)
}
```

`register_typed` delegates to `register_aip_fn`.

The generic registration flow is:

1. Create a Lua function with `lua.create_function`.
2. Require a single params argument.
3. Convert the Lua value to `F::Params` through `AipFromLua`.
4. Call the Rust handler.
5. Convert `F::Response` to Lua through `AipToLua`.
6. Convert `F::Error` to Lua error through `IntoAipLuaError`.
7. Register the function on the Lua table using `F::NAME`.

## Params conversion

`lua_params_from_value` converts the incoming Lua value to a serde value, then deserializes it into the params type.

Behavior:

- A Lua table is converted into a serde object or array according to the existing Lua-to-serde helper behavior.
- An empty Lua table is treated as an empty JSON object.
- Non-table values are converted through the generic Lua-to-serde helper.
- Deserialization errors become Lua runtime errors with `INVALID_PARAMS`.

The empty table special case allows APIs with optional fields to accept:

```lua
aip.json.parse({})
```

## Success response conversion

`return_success_envelope` serializes the response struct to serde JSON, then converts it to Lua.

Because response structs already contain the `data` field, the returned Lua table has the expected root-level API shape.

Example response struct:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipJsonStringifyResult {
	pub data: String,
}
```

Lua result:

```lua
{
  data = "{\"name\":\"AIProg\"}"
}
```

## Defining a new AipFn function

Use this checklist when adding a new AI-facing Lua function.

### 1. Define the marker struct

```rust
pub struct AipExampleEchoFn;
```

### 2. Implement `AipFn`

```rust
impl AipFn for AipExampleEchoFn {
	const NAME: &'static str = "echo";
	type Params = AipExampleEchoParams;
	type Response = AipExampleEchoResult;
	type Error = AipApiError;
}
```

### 3. Define params

```rust
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipExampleEchoParams {
	pub data: String,
}
```

Use `data` for the primary input payload.

Use additional fields only for metadata, options, pagination, or other non-primary payload values.

### 4. Define response

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipExampleEchoResult {
	pub data: String,
}
```

Use `data` for the primary output payload.

### 5. Define handler

```rust
fn aip_example_echo_handler(params: AipExampleEchoParams) -> core::result::Result<AipExampleEchoResult, AipApiError> {
	Ok(AipExampleEchoResult {
		data: params.data,
	})
}
```

### 6. Register the function

```rust
AipExampleEchoFn::register_typed(lua, &table, aip_example_echo_handler)?;
```

## Current `aip.json` functions

The `aip.json` module uses the `AipFn` pattern for all functions.

File:

- `src/script/modules/aip_json.rs`

Functions:

- `aip.json.parse(params: { data?: string }) -> { data: any }`
- `aip.json.parse_ndjson(params: { data?: string }) -> { data: any[] }`
- `aip.json.stringify(params: { data: any }) -> { data: string }`
- `aip.json.stringify_pretty(params: { data: any }) -> { data: string }`

Marker structs:

- `AipJsonParseFn`
- `AipJsonParseNdjsonFn`
- `AipJsonStringifyFn`
- `AipJsonStringifyPrettyFn`

The module registers all functions through `AipFn::register_typed`.

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
- `IntoAipLuaError`

`AipApiError` is the default error type unless a function needs a specialized error type.

## Naming conventions

Use names that match the Lua API path.

For a Lua function:

```lua
aip.json.stringify_pretty(...)
```

Use:

- marker struct: `AipJsonStringifyPrettyFn`
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
- Keep Lua conversion in the shared `AipFn` support layer.
- Avoid `.unwrap()` and `.expect(...)`.
- Preserve root-level response shape, do not introduce a `result` wrapper.
- Use `data` as the primary payload field.
- Add schema derives to params and response types.
- Prefer a marker struct and typed handler for every AI-facing function.
- Keep marker structs near their related params, response, and handler definitions.
- Do not reintroduce legacy positional APIs for AI-facing functions unless explicitly required for backward compatibility.
