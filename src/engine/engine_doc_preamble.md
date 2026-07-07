# AIP Script Engine API

This document describes the API for all registered handlers available in the AIP Script Engine runtime. Each handler is callable from scripts using the `namespace.module.function` naming convention.

## Function Signatures

Throughout this document, function signatures use a TypeScript-like notation:

```
namespace.module.function(params: Params): ReturnType
```

- `Params` is an object type detailing the expected input for the handler.
- `ReturnType` is the handler's return type. For simple types (`string`, `number`, `boolean`, `Array<T>`, etc.), the return type appears inline in the signature. For object types, a separate `type Output = { ... };` block follows the signature.
- Unless otherwise specified, handlers returning a `HandlerResult<T>` resolve to the declared return type on success or an error on failure.

## Common Error Type

All handlers in this engine return errors using the standard structure:

```ts
type Error = {
  message: string;
};
```

This common error shape applies to every handler in this document unless a handler explicitly documents a custom `type Error = ...` block.

> **Note:** Handlers that use the common error pattern omit the `type Error` block from their individual documentation. The definition above applies to all such handlers.

## Shared Types

Reusable types shared across multiple handlers (e.g., `FileGlobs`, `FileInfo`) are defined once in the `## Shared Types` section at the end of this document. Handlers reference these types by name in their signatures.


## Additional Lua Native Functions

In addition to the handlers exposed by registered modules, the Lua runtime provides a set of global utility functions. These are installed by the engine at initialization and are available in every script.

The following functions follow the same signature notation outlined in [Function Signatures](#function-signatures). All are global functions and do not return errors.

- `null` / `Null` / `NULL` — global constants that hold the engine's null sentinel. This sentinel is distinct from Lua's built-in `nil` and can be tested with the null-checking functions below.

- `is_null(x: any): boolean` — returns `true` if `x` is `nil` or the engine's null sentinel.

- `is_not_null(x: any): boolean` — returns `true` if `x` is neither `nil` nor the null sentinel.

- `nil_if_null(x: any): any` — returns `x` if it is a meaningful value; otherwise returns `nil`. Useful for converting the null sentinel into Lua's `nil` for standard operations.

- `value_or(...values: any[]): any` — returns the first argument that is not null or the engine's null sentinel; returns `nil` if all arguments are null/nil or no arguments are provided. Provides a fallback pattern similar to `??` in many languages.

- `is_table(x: any): boolean` — returns `true` if `x` is a Lua table (excluding `nil` and null).

- `is_list(x: any): boolean` — returns `true` if `x` is a Lua table that appears to be an array-like list (its first numeric index is not `nil`).

- `is_object(x: any): boolean` — returns `true` if `x` is a Lua table that appears to be a dictionary/object (its first numeric index is `nil`).

