# AIProg

AIProg is a Rust runtime for executing constrained Lua programs against explicitly registered, Rust-backed APIs.

The crate is designed for applications that want an AI system, or another program author, to orchestrate approved capabilities as a small Lua program instead of making individual tool calls.

## Primary entry points

- [`EngineTemplate`](crate::EngineTemplate) creates isolated script execution environments from an [`AipRegistry`](crate::AipRegistry). It is the preferred API for handlers that need execution-scoped state.

- [`ScriptEngine`](crate::ScriptEngine) executes scripts with context-free handler registries. Use [`ScriptEngine::new_context_free`](crate::ScriptEngine::new_context_free) or [`ScriptEngine::from_context_free_registry`](crate::ScriptEngine::from_context_free_registry) only when no registered handler needs a [`RunningContext`](crate::RunningContext).

- [`AipRegistryBuilder`](crate::AipRegistryBuilder) registers synchronous and asynchronous handlers, combines modules, and builds an immutable [`AipRegistry`](crate::AipRegistry).

- [`AipModule`](crate::AipModule) provides composable registration for a group of handlers. Built-in modules include [`JsonModule`](crate::modules::JsonModule), [`WebModule`](crate::modules::WebModule), [`FileModule`](crate::modules::FileModule), and [`HtmlModule`](crate::modules::HtmlModule).

## Execution with context

Use [`EngineTemplate`] when handlers need caller-provided capabilities or state. Insert values into a [`RunningContext`] before execution, then recover them from the returned [`RunOutcome`].

```rust
use aiprog::{AipRegistry, EngineTemplate, RunningContext};

let template = EngineTemplate::builder()
	.with_registry(AipRegistry::from_empty())
	.build()?;

let outcome = template
	.exec("return { message = 'hello' }", RunningContext::default())
	.await?;

let value = outcome.result?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Handlers receive a [`HandlerCallContext`] and can access typed values in the current [`RunningContext`]. Applications commonly insert capability policies, service clients, or request-specific state into the context before starting an engine.

## Registering handlers

A handler accepts a [`HandlerCallContext`], a strongly typed parameter value implementing [`AipParams`], and returns a [`HandlerResult`](crate::HandlerResult) containing an output type implementing [`AipOutput`].

Use the [`aip_handler`](crate::aip_handler) attribute and [`register_handler`](crate::register_handler) macro for generated handler metadata and registration support. For lower-level registration, use [`AipRegistryBuilder::register_sync`] or [`AipRegistryBuilder::register_async`].

## Filesystem capabilities

The built-in file module requires a [`DirContext`](crate::DirContext) in the running context. Construct it with separate read and write [`PathPolicy`](crate::PathPolicy) values. Each policy defines canonical allowed roots and whether absolute paths are permitted through [`AbsolutePathPolicy`](crate::AbsolutePathPolicy).

This explicit capability model prevents a script from obtaining filesystem access outside roots supplied by the host application.

## Error handling

Most public APIs return [`Result`](crate::Result), whose error type is [`Error`](crate::Error). Template startup and execution preserve ownership of the caller's context when possible through [`EngineStartError`](crate::EngineStartError), [`RunningEngineFinishError`](crate::RunningEngineFinishError), and [`TemplateExecutionError`](crate::TemplateExecutionError).

Use [`RunOutcome::into_parts`](crate::RunOutcome::into_parts) when both the script result and recovered context need to be handled together.

## Schema inspection

[`SchemaRef`](crate::SchemaRef) and [`SchemaPropRef`](crate::SchemaPropRef) provide borrowed convenience views over `schemars` schemas. They are useful for consumers that generate documentation or UI from registered handler schemas.

## Feature organization

- [`registry`](crate::registry) contains handler registration, schemas, handler errors, and registry selection.
- [`schema_ref`](crate::schema_ref) contains read-only schema inspection helpers.
- [`modules`](crate::modules) exposes the built-in module marker types and filesystem policy types.
- [`webc`](crate::webc) provides the underlying web client abstractions.
- [`types`](crate::types) contains public supporting types.

The Lua runtime's built-in functions and registered handlers are implementation details of the selected registry and modules. Rustdoc documents the Rust API used to configure and host that runtime.
