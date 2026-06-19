# LuaExt Trait (Lua Value Extraction)

## When to use

Use the `LuaExt` trait and its `x_get_*` methods whenever Rust code needs to read values from Lua `Value`s or `Table`s. These extensions provide a convenient, nil-aware layer over `mlua`'s raw accessors, avoiding common pitfalls like panics on nil values or unexpected types. Prefer these methods over direct `mlua` table indexing or value conversion.

## Intent

Provide a set of extension methods for `mlua::Value` and `mlua::Table` that simplify reading typed values from Lua data structures. The extensions handle nil values, missing keys, and type coercions (e.g., integer rounding from floats) in a consistent way, returning `Option` to allow graceful error handling in the caller.

This module is the primary interface between Rust and Lua for reading configuration, request parameters, and any Lua-table-shaped data within the crate.

## Public API

### Trait: `LuaExt`

Implemented for both `mlua::Value` and `mlua::Table`.

```rust
pub trait LuaExt {
    fn x_is_null(&self) -> bool;
    fn x_as_lua_str(&self) -> Option<BorrowedStr<'_>>;
    fn x_as_i64(&self) -> Option<i64>;
    fn x_as_f64(&self) -> Option<f64>;
    fn x_to_string(&self) -> Option<String>;
    fn x_get_value(&self, key: &str) -> Option<Value>;
    fn x_get_string(&self, key: &str) -> Option<String>;
    fn x_get_bool(&self, key: &str) -> Option<bool>;
    fn x_get_i64(&self, key: &str) -> Option<i64>;
    fn x_get_f64(&self, key: &str) -> Option<f64>;
    fn x_as_list(&self) -> Option<Vec<Value>>;
}
```

### Key accessor methods (`x_get_*`)

These methods are designed for reading fields from Lua tables:

- `x_get_value(key) → Option<Value>`: returns the raw `Value` for the given key, or `None` if the key is missing, the value is `nil`, or `self` is not a table.
- `x_get_string(key) → Option<String>`: reads the value as a Lua string (lossy conversion to `String`). Returns `None` if not a string or not a table.
- `x_get_bool(key) → Option<bool>`: reads the value as a boolean.
- `x_get_i64(key) → Option<i64>`: reads the value as an integer (or integer-rounded float).
- `x_get_f64(key) → Option<f64>`: reads the value as a float (promoting integers).

These methods first attempt to access the table by `key`, then attempt the appropriate conversion, returning `None` at any failure point.

### Direct value accessors

- `x_as_lua_str(&self) → Option<BorrowedStr>`: borrows the value as a Lua string.
- `x_as_i64(&self) → Option<i64>`: interprets the value as an integer, rounding floats.
- `x_as_f64(&self) → Option<f64>`: interprets the value as a float.
- `x_to_string(&self) → Option<String>`: converts the value to an owned string.
- `x_is_null(&self) → bool`: returns true if the value is `Value::Null` or `Value::Nil`.
- `x_as_list(&self) → Option<Vec<Value>>`: extracts the sequential part of a table (1..n contiguous, stops at first nil).

## Code Design

The trait is defined in `src/script/lua_exts/lua_ext.rs`, with a companion helper `table_as_list`. The trait is implemented for `mlua::Value` and `mlua::Table` separately, with the `Table` impl providing some overrides (e.g., `x_is_null` returns `false`).

The module hierarchy:

```
src/script/lua_exts/
  mod.rs           – re-exports LuaExt, LuaJsonExt, and other traits
  lua_ext.rs       – LuaExt trait and impls
  lua_json_ext.rs  – (adjacent) JSON conversion extensions
  lua_traits.rs     – (adjacent) other traits
```

The trait methods are all infallible in the RustResult sense: they return `Option`, never panic, and never produce a Lua error. This allows callers to chain access patterns with `?` on Option and gracefully fallback.

## Design Considerations

- **Nil-safety**: The `x_get_*` methods treat `nil` as absence, returning `None`. This matches common Lua patterns where a missing key is indistinguishable from a key set to `nil`.
- **Type coercion**: `x_as_i64` rounds floating numbers; `x_as_f64` promotes integers to float. This mimics Lua’s number flexibility but in a typed Rust context.
- **No error details**: The `Option` return hides the reason for failure (e.g., wrong type vs missing key). For debugging, callers may use raw mlua access. This is acceptable for most configuration and request parsing use cases.
- **Performance**: The trait adds a thin layer of indirection but avoids unnecessary cloning when possible (e.g., `x_as_lua_str` returns a borrow).
- **Future extensions**: Additional helper methods (e.g., `x_get_date`, `x_get_array_of`) could be added as new trait methods or as free functions that build on `x_get_*` primitives.
