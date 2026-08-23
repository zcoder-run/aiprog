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

Global Lua utilities available in every script.

```ts
null / Null / NULL // engine null sentinel, distinct from nil

is_null(x: any): boolean
is_not_null(x: any): boolean
nil_if_null(x: any): any
value_or(...values: any[]): any // first non-nil/non-null value

is_table(x: any): boolean
is_list(x: any): boolean   // array-like table
is_object(x: any): boolean // object-like table

merge(
  target?: table|nil|null,
  ...sources: table|nil|null
): table|nil // shallow, mutates target; nil/null inputs are skipped

merge_deep(
  target?: table|nil|null,
  ...sources: table|nil|null
): table|nil // recursive, mutates target; nil/null inputs are skipped
```

`merge*` uses the first non-nil/non-null table as the target when needed and errors on invalid source types.