# Handler and Registry Scheme

## Intent

The handler scheme provides a type-safe bridge between Rust functions and Lua scripts. It defines:

- A central registry (`AipRegistry`) for registering functions under hierarchical paths (`namespace.module.function`) and invoking them with Lua data.
- A strict contract: each handler receives a single typed parameters struct (`AipParams`) and returns a typed result (`AipOutput`) or a typed error (`AipError`), with automatic schema generation.
- Marker traits that enforce Lua/Rust conversion boundaries, keeping the two domains decoupled.
- Flexible registration: handlers can be registered manually or via the `#[aip_handler]` attribute macro.

## Registry and Handler Context

The `AipRegistry` maintains a collection of registered function entries. Each entry maps a unique function path to a handler closure and associated JSON schemas for its parameters, output, and error types.

A handler is a Rust implementation that bridges typed Rust data and Lua values. Handlers are invoked by the registry, which passes deserialized parameters from Lua, runs the handler logic, and converts the typed result back to a Lua value or raises a normalized error. The registry decouples function dispatch from Lua execution, providing a centralized mechanism for discovering, invoking, and documenting API functions.

## Function Path Scheme

All registered functions follow a hierarchical naming convention:

`<namespace>.<module>.<function>`

- `namespace`: The top-level scope. Built-in functions use `aip` (e.g., `aip.json.parse`). Users can also register functions under custom namespaces or share the `aip` namespace.
- `module`: Groups related functions (e.g., `json`, `web`, `file`).
- `function`: The specific operation (e.g., `parse`, `get`).

Each function call adheres to a strict input/output contract:
- **Input**: A single typed `Params` argument (deserialized from a Lua table).
- **Output**: A typed `Output` value (converted back to a Lua table/value), or a typed `Error` if the operation fails.

Example:
```lua
-- Lua usage for a built-in function
local result = aip.json.parse({ text = '{"key": "value"}' })
-- result is the typed Output or throws a typed Error
```

## Type System and Marker Traits

The handler framework uses marker traits to enforce type safety and schema generation. Concrete parameter, output, and error types must implement these traits to be accepted in the handler.

### `AipParams` (Input Type)
- Enforces that a type can be deserialized from Lua (`AipFromLua`).
- Requires `schemars::JsonSchema` so the registry can store and expose the parameter schema.
- Requires `Send + Sync + 'static` for thread safety across the async runtime.

### `AipOutput` (Output Type)
- Enforces that a type can be serialized to Lua (`AipIntoLua`).
- Requires `schemars::JsonSchema` for schema generation.
- Requires `Send + Sync + 'static` for thread safety.

### `AipError` (Error Type)
- Marks types that can be converted into a normalized `HandlerError` and propagated to Lua.
- Requires `schemars::JsonSchema` so the error schema is included in the registry entry.

When registering a function, the generic bounds `P: AipParams`, `O: AipOutput`, and the error type implementing `AipError` ensure that the closure logic remains decoupled from Lua while guaranteeing type-safe conversions at the registry boundary.

## Handler Function Signature

A handler is a Rust function that implements the business logic for a registry entry.

```rust
fn handler_name(call: HandlerCallContext, params: P) -> HandlerResult<O>
// or for async
async fn handler_name(call: HandlerCallContext, params: P) -> HandlerResult<O>
```

- `P`: The concrete `Params` type.
- `O`: The concrete `Output` type.
- `HandlerCallContext`: Scoped access to the current execution's typed running context.
- `HandlerResult<O>`: A type alias for `Result<O, HandlerError>`.

The conversion from Lua to `P` and from `O` to Lua is handled automatically by the registration macros and closures.

## Context-Free Script Engines

`EngineTemplate` is the required execution path for registries containing handlers that access `RunningContext`, such as the built-in file handlers that require `DirContext`.

`LuaEngine::new_context_free` and `LuaEngine::from_context_free_registry` support legacy context-free execution only. Their handler calls use an empty context, so a context-dependent handler returns the typed missing-context error when invoked. `LuaEngine::new` and `LuaEngine::from_registry` retain this legacy behavior for compatibility; new code should use the explicitly named context-free constructors or an `EngineTemplate` as appropriate.

## Registration Methods

There are two ways to register a function in the `AipRegistry`: manual registration and attribute-based registration. Both produce identical registry entries with schemas and closures.

### 1. Manual Registration (`register_sync` / `register_async`)

Handlers are passed directly as closures or stateful objects. This approach is useful for complex handlers, shared state, or custom lifecycle management.

```rust
let registry = AipRegistryBuilder::default()
	.register_sync::<MyParams, MyOutput, _>("namespace.module.sync_func", sync_handler)?
	.register_async::<MyParams, MyOutput, _>("namespace.module.async_func", async_handler)?
	.build();
```

`register_sync` and `register_async` are the canonical manual registration APIs. They consume and return the builder, so registration can be chained before `build` produces an immutable `AipRegistry`. The registry creates the necessary `LuaSyncClosure` or `LuaAsyncClosure` internally, while the caller provides the typed handler directly.

### 2. Attribute-Based Registration (`#[aip_handler]` + `register_handler`)

Use this approach when the `#[aip_handler]` proc-macro's generated metadata and signature validation are desired. The macro analyzes the function signature, extracts documentation metadata, and generates a hidden unit struct that implements the `AipHandler` trait.

```rust
/// # My Function
/// Description here.
#[aip_handler]
async fn my_handler(call: HandlerCallContext, params: MyParams) -> HandlerResult<MyOutput> {
    // logic
}
```

Register macro-generated handlers through `register_handler`:

```rust
let mut builder = AipRegistryBuilder::default();
register_handler!(builder, "namespace.module.func", my_handler)?;
let registry = builder.build();
```

`register_handler` is reserved for values implementing `AipHandler`, which are normally generated by `#[aip_handler]`. The macro handles schema generation, closure boxing, and metadata extraction automatically.

## Registry Composition

`AipRegistry` is immutable. Use `select` or `exclude` to derive a registry with a different API surface, then call `to_builder` to extend or merge that registry.

```rust
let selected = full_registry.select(
	["aip.json.*", "aip.file.read"],
	RegistrySelectionOptions::default(),
)?;

let registry = selected
	.to_builder()
	.register_sync("app.lookup", app_lookup_handler)?
	.merge(application_registry)?
	.build();
```

Both selection methods require `RegistrySelectionOptions`. Use `RegistrySelectionOptions::default()` to allow valid patterns that match no registered functions, or set `unmatched_patterns` to `UnmatchedPatternPolicy::Error` when a capability declaration must match every pattern.

## Design Considerations

- **Unified Input Contract**: Passing a single typed table simplifies Lua API calls, allows optional fields, and avoids positional argument ambiguity.
- **Typed Boundaries**: Marker traits (`AipParams`, `AipOutput`, `AipError`) enforce strict type conversions at the registry boundary, keeping Lua and Rust domains cleanly separated while providing automatic schema generation for documentation and validation.
- **Registration Flexibility**: Offering both manual and attribute-based registration balances developer control (for advanced cases) with reduced boilerplate (for standard cases).
- **Path Hierarchies**: The `<namespace>.<module>.<function>` scheme provides a clear, scalable structure for both built-in and user-defined APIs.
