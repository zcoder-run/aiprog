# Isolated Script Engine Architecture

## Intent

Provide isolated Lua script execution through reusable immutable configuration and per-run state.

`ScriptEngine` stores the reusable execution configuration. Each execution creates a fresh Lua VM, binds the selected registry's handlers to one owned `RunningContext`, executes a script, drops the VM, and recovers the context.

The implementation separates:

- Immutable API definitions in `AipRegistry`.
- Lua runtime configuration in `ScriptEngine`.
- Per-execution typed Rust state in `RunningContext`.
- One-run Lua state in `RunningEngine`.

[Lifecycle](#execution-lifecycle) | [Architecture](#architecture-at-a-glance) | [Registry](#registry-architecture) | [Context](#running-context) | [Runtime policy](#lua-runtime-policy) | [Errors](#error-and-context-recovery) | [Current limits](#current-limitations)

## Example

```rust
let registry = AipRegistryBuilder::default()
	.add_module(JsonModule)?
	.add_module(WebModule)?
	.build();

let engine = ScriptEngine::builder()
	.with_registry(registry)
	.build()?;

let mut context = RunningContext::default();
context.insert(String::from("execution state"));

let outcome = engine
	.exec("return 42", context)
	.await?;

let value = outcome.result?;
let context = outcome.context;
```

`outcome.result` contains either the script result or the script error. The `RunningContext` is recovered in either case when Lua state destruction and context recovery both succeed.

## Architecture at a glance

```mermaid
flowchart TD
	subgraph reusable["Reusable configuration"]
		builder["AipRegistryBuilder"]
		registry["AipRegistry<br/>immutable handler definitions"]
		engine["ScriptEngine<br/>reusable isolated-run configuration"]

		builder --> registry
		registry --> engine
	end

	subgraph execution["One execution"]
		context["RunningContext<br/>owned typed values"]
		handle["RunningContextHandle"]
		running["RunningEngine<br/>one template-created execution"]
		lua["Fresh Lua VM"]
		handlers["Bound handler closures"]
		outcome["RunOutcome<br/>result and recovered context"]

		context --> handle
		engine --> running
		handle --> running
		running --> lua
		running --> handlers
		lua --> outcome
		handle --> outcome
	end

```

The reusable layer contains only immutable registry definitions, runtime policy, and native function installers. A run owns its Lua state and execution context. The context is recovered only after the Lua state and its closures have been dropped.

## ScriptEngine and LuaEngine

`ScriptEngine` is the public reusable engine configuration. `LuaEngine` is its private concrete Lua runtime implementation.

- `ScriptEngine` is the reusable, cloneable public engine configuration. It stores an `AipRegistry`, a `LuaRuntimePolicy`, and approved native function installers, but it does not own a Lua VM or a `RunningContext`.
- `ScriptEngine::start` creates a `RunningEngine` for one isolated execution. That running engine owns a fresh private `LuaEngine`, its fresh Lua VM, the bound registry closures, and the supplied `RunningContext` handle.
- `LuaEngine` is an implementation detail that constructs and executes the concrete Lua VM used by `RunningEngine`.

Use `ScriptEngine` for isolated executions that need per-run context recovery and reusable policy configuration. It binds context-dependent handlers to the supplied `RunningContext`.

## Execution lifecycle

The implemented execution lifecycle is:

1. Build an `AipRegistry` containing reusable handler definitions.
2. Build a `ScriptEngine` with the registry, Lua policy, and optional native functions.
3. Supply an owned `RunningContext` to `ScriptEngine::start` or `ScriptEngine::exec`.
4. Create a fresh restricted Lua VM.
5. Create a `HandlerCallContext` bound to the supplied context.
6. Bind registry handler factories to that call context.
7. Install native functions and bound handlers into Lua.
8. Execute the Lua script.
9. Drop the `LuaEngine`, Lua VM, and bound closures.
10. Recover and return the `RunningContext`.

`ScriptEngine::start` exposes the explicit lifecycle:

```rust
let running = engine.start(context)?;
let outcome = running.exec(script).await?;
```

`ScriptEngine::exec` is the convenience operation that starts and executes a one-run engine:

```rust
let outcome = engine.exec(script, context).await?;
```

`RunningEngine::exec` consumes `self`, ensuring that one running engine is used for only one execution and allowing its Lua state to be dropped before context recovery.

## Registry architecture

`AipRegistry` is an immutable, cloneable collection of reusable handler definitions. It shares definitions through `Arc<HandlerDefinition>` values rather than rebuilding schemas and handler factories for each run.

Each definition contains:

- Its dotted Lua path.
- Its synchronous or asynchronous kind.
- Parameter, output, and error schemas.
- Optional title and description metadata.
- A factory that binds a `HandlerCallContext` to a per-run Lua closure.

`AipRegistryBuilder` is the mutable construction API. It owns its current definition list and validates paths while definitions are added.

```rust
let base = AipRegistryBuilder::default()
	.add_module(JsonModule)?
	.add_module(WebModule)?
	.build();

let selected = base.select(
	["aip.json.*"],
	RegistrySelectionOptions {
		unmatched_patterns: UnmatchedPatternPolicy::Error,
	},
)?;

let extended = selected
	.to_builder()
	.merge(application_registry)?
	.build();
```

Registry composition has value semantics:

- `select` returns matching definitions in source registration order.
- `exclude` returns definitions not matching the supplied patterns.
- `to_builder` creates an independent builder while reusing shared definitions.
- `merge` preserves destination order, then appends source order.
- Duplicate paths are rejected.
- Existing registries are never mutated.

Pattern matching supports literal segments, `*` for one segment, and `**` for zero or more segments.

## Handler binding

Handler definitions are reusable, but Lua closures are execution-specific because they capture the current `HandlerCallContext`.

Before installing functions into Lua, `AipRegistry::bind` creates an internal `BoundRegistry`. Each entry contains a Lua closure generated from its immutable handler definition and the per-run call context.

The engine installs synchronous handlers with `Lua::create_function` and asynchronous handlers with `Lua::create_async_function`. Handler outputs are converted directly to Lua values. The script result is converted to `serde_json::Value` only at the script engine's public result boundary.

The private `LuaEngine` is created by `ScriptEngine` for each run. Context-dependent handlers are bound to the supplied `RunningContext`.

## Running context

`RunningContext` is an owned type map keyed by Rust `TypeId`. It stores values that are `Any + Send + Sync + 'static`.

It provides typed insertion, retrieval, mutable retrieval, and removal:

```rust
let mut context = RunningContext::default();

context.insert::<u32>(42);
assert_eq!(context.get::<u32>(), Some(&42));

context.get_mut::<u32>().map(|value| *value += 1);
assert_eq!(context.remove::<u32>(), Some(43));
```

A `HandlerCallContext` exposes short scoped access to values while handlers execute:

```rust
let result = call_context.with::<MyService, _>(|service| {
	service.describe()
})?;

call_context.with_mut::<AuditLog, _>(|audit| {
	audit.record("handler called");
})?;
```

Internally, a `RunningContextHandle` uses `Arc<Mutex<RunningContext>>` while bound Lua closures exist. `HandlerCallContext::with` and `HandlerCallContext::with_mut` map missing values and poisoned locks to `ContextAccessError`.

Handlers must not retain `HandlerCallContext` after their work is complete. Retained handles prevent exclusive ownership recovery and produce `ContextRecoveryError::OutstandingHandles`.

## Lua runtime policy

`LuaRuntimePolicy` defines the standard libraries and resource settings used for each fresh VM.

The default policy enables:

- Base.
- Coroutine.
- Math.
- String.
- Table.
- UTF-8.

The default policy disables:

- Package.
- IO.
- OS.
- Debug.

The base library is required. `ScriptEngineBuilder::build` rejects a policy that disables it.

```rust
let policy = LuaRuntimePolicy::default()
	.with_std_lib_policy(
		LuaStdLibPolicy::default()
			.with_package(false)
			.with_io(false)
			.with_os(false)
			.with_debug(false),
	)
	.with_limits(
		LuaExecutionLimits::default()
			.with_max_memory_bytes(16 * 1024 * 1024),
	);
```

`LuaExecutionLimits` supports configuring a maximum memory size, maximum instruction count, and wall-clock timeout. The current runtime applies memory limits through `mlua::Lua::set_memory_limit`.

`NativeFunctionSet` separately holds explicitly approved Rust installers. Native installers run after the Lua VM and registered handler functions have been created.

## Error and context recovery

The engine distinguishes script errors from lifecycle errors.

`RunOutcome<T>` contains:

```rust
pub struct RunOutcome<T, E = crate::Error> {
	pub result: core::result::Result<T, E>,
	pub context: RunningContext,
}
```

Compilation and runtime failures are stored in `RunOutcome::result`, allowing the recovered context to remain available.

If engine setup fails, `ScriptEngine::start` returns `EngineError::Start`, which contains an `EngineStartError`:

- `Setup` contains the setup source error and recovered `RunningContext`.
- `ContextRecovery` reports both the setup source and a context recovery failure.

If execution finishes but the context cannot be recovered, `RunningEngine::exec` returns `RunningEngineFinishError<T>`. It retains the script result alongside the recovery error.

`EngineError` wraps build, start, and finish-recovery errors for the convenience `ScriptEngine::exec` API.

## Isolation guarantees

Every `ScriptEngine::start` creates a fresh Lua VM. Therefore, Lua globals created by one execution are not available to later executions from the same engine.

The template can be cloned and shared because it contains immutable configuration. Each concurrent execution receives its own:

- Lua VM.
- Bound Lua closures.
- `RunningContextHandle`.
- Owned `RunningContext`.

The implementation includes tests confirming that globals do not leak across template executions and that context is returned after script and supported setup failures.

## Current limitations

The following constraints are intentionally enforced or remain unresolved:

- Instruction limits are configurable but currently rejected as unsupported.
- Wall-clock timeouts are configurable but currently rejected as unsupported.
- Cancellation does not yet have a dedicated public finish and context-recovery protocol.
- Context recovery requires all `HandlerCallContext` clones to be dropped.
- `LuaEngine` is private and is created only as part of a `ScriptEngine` execution.
- Lua policy and native function installation are separate controls. Restricting registry functions alone is not a complete sandbox.

## Implementation files

- `src/engine/script_engine/script_engine_impl.rs`, `ScriptEngine` configuration, fresh Lua VM creation, start and execution lifecycle.
- `src/engine/script_engine/lua_runtime_policy.rs`, standard-library and execution-limit policy types.
- `src/engine/script_engine/error.rs`, build, start, and context-recovery lifecycle errors.
- `src/engine/support/lua_engine.rs`, lower-level `LuaEngine` construction and execution APIs.
- `src/engine/support/lua_engine_register.rs`, registry binding and Lua function installation.
- `src/running_context.rs`, typed per-run state, scoped handler access, and recovery.
- `src/run_outcome.rs`, script result and recovered context container.
- `src/registry/registry_impl.rs`, immutable registry composition and binding.
- `src/registry/registry_types.rs`, registry metadata, handler wrapper traits, and selection errors.
