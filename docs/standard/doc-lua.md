# Lua AIP Extensions

This document describes the built-in Lua extensions provided by the AIP runtime.  
They are available globally in every Lua script executed by the engine.

## Null Sentinel

AIP exposes a special **null sentinel** value to represent explicit `null` as distinct from Lua's `nil`.  
This sentinel is bound to three global variables:

- `null`
- `NULL`
- `Null`

All three point to the same sentinel (`mlua::Value::NULL`).  
Use them interchangeably.

**Example:**

```lua
local val = null
print(is_null(val))  --> true
```

## Global Helper Functions

The following functions are available in the global scope.

### `is_null(value)`

Returns `true` if `value` is the null sentinel or Lua `nil`.

```lua
is_null(null)       --> true
is_null(nil)        --> true
is_null(42)         --> false
```

### `is_not_null(value)`

Returns `true` if `value` is **not** the null sentinel and **not** Lua `nil`.

```lua
is_not_null(nil)    --> false
is_not_null(null)   --> false
is_not_null("hi")   --> true
```

### `nil_if_null(value)`

Returns `nil` if `value` is the null sentinel; otherwise returns `value` unchanged.

```lua
nil_if_null(null)   --> nil
nil_if_null("text") --> "text"
```

### `value_or(value, default)`

Returns `value` if it is not `nil` and not the null sentinel; otherwise returns `default`.

```lua
value_or(nil, "fallback")  --> "fallback"
value_or(null, 0)          --> 0
value_or("ok", "fallback") --> "ok"
```

### `is_table(value)`

Returns `true` if `value` is a Lua table (and not nil or the null sentinel).

```lua
is_table({})        --> true
is_table(nil)       --> false
is_table(null)      --> false
```

### `is_list(value)`

Returns `true` if `value` is a Lua table and has an integer key `1` (i.e., it behaves like a sequential list).

```lua
is_list({10, 20})   --> true
is_list({a=1})      --> false
```

### `is_object(value)`

Returns `true` if `value` is a Lua table and does **not** have an integer key `1` (i.e., it behaves like a dictionary).

```lua
is_object({a=1})    --> true
is_object({10, 20}) --> false
```

### `merge(target, ...sources)`

Merges table arguments into a target table. The target may be `nil` or `null`; if so, the function skips leading `nil`/`null` arguments and uses the first non‑`nil`, non‑`null` table as the target. All subsequent arguments that are tables have their key-value pairs set on the target, overwriting existing keys. Arguments that are `nil` or `null` are skipped.

Returns the target table (modified in place), or `nil` if no non‑`nil`/non‑`null` table was provided.

```lua
local t = {a = 1}
merge(t, {b = 2}, {a = 99})
-- t is now {a = 99, b = 2}

-- nil first argument
local r = merge(nil, {x = 1}, {y = 2})
-- r is now {x = 1, y = 2}
```

If any argument after the target is not a table, `nil`, or `null`, an error is raised.

### `merge_deep(target, ...sources)`

Deep merges table arguments into a target table. The target may be `nil` or `null`; if so, the function skips leading `nil`/`null` arguments and uses the first non‑`nil`, non‑`null` table as the target. When both `target[k]` and `source[k]` are tables, they are recursively deep‑merged; otherwise `target[k]` is overwritten by `source[k]`. Arguments that are `nil` or `null` are skipped.

Returns the target table (modified in place), or `nil` if no non‑`nil`/non‑`null` table was provided.

```lua
local t = {a = {x = 1}}
merge_deep(t, {a = {y = 2}, b = 3})
-- t is now {a = {x = 1, y = 2}, b = 3}

-- nil first argument
local r = merge_deep(nil, {a = {x = 1}}, {a = {y = 2}})
-- r is now {a = {x = 1, y = 2}}
```

If any argument after the target is not a table, `nil`, or `null`, an error is raised.

## Module Extensions

Beyond the global helpers, AIP provides higher-level modules installed under the `aip` namespace:

- **`aip.json`** – JSON parsing and stringification.  
  See `doc-aip-json.md`.
- **`aip.web`** – HTTP client (`get`, `post`).  
  See `doc-aip-web.md`.

Each module follows the [aip API scheme](dev/specs/spec-aiprog-api-scheme.md): functions accept a single table of named parameters and return either a direct value or a result table with a `data` field.

## Execution Context

Handlers that require execution-scoped Rust state, including `aip.file` handlers that require a `DirContext`, must run through `EngineTemplate` with a caller-supplied `RunningContext`.

`ScriptEngine::new_context_free` and `ScriptEngine::from_context_free_registry` are for context-free APIs. Calling a context-dependent handler through those APIs returns a predictable missing-context error rather than granting a fallback capability.

## Rust-side Extensions (Internal)

The crate also provides Rust helper traits (`LuaExt`, `LuaJsonExt`) for extracting values from Lua tables and converting between JSON and Lua.  
These traits are not directly accessible from Lua scripts; they are used internally by the `aip.*` modules and other native code.

For reference, see the implementation in `src/script/lua_exts/`.
