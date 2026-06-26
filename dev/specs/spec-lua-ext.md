# LuaExt Trait (Lua Value Extraction)

## When to use

Use the `LuaExt` trait and its `x_get_*` methods whenever Rust code needs to read values from Lua `Value`s or `Table`s. These extensions provide a convenient, nil-aware layer over `mlua`'s raw accessors, avoiding common pitfalls like panics on nil values or unexpected types. Prefer these methods over direct `mlua` table indexing or value conversion. Handler implementations for the `aip.*` Lua API (see [`dev/specs/spec-handler-scheme.md`](spec-handler-scheme.md)) rely on these methods to extract typed parameters from the single-table argument.

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
    fn x_as_bool(&self) -> Option<bool>;
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
- `x_as_bool(&self) → Option<bool>`: interprets the value as a boolean.
- `x_is_null(&self) → bool`: returns true if the value is `Value::Null` or `Value::Nil`.
- `x_as_list(&self) → Option<Vec<Value>>`: extracts the sequential part of a table (1..n contiguous, stops at first nil).

## Code Design

The trait is defined in `src/lua_exts/lua_ext.rs`, with a companion helper `table_as_list`. The trait is implemented for `mlua::Value` and `mlua::Table` separately, with the `Table` impl providing some overrides (e.g., `x_is_null` returns `false`).

The module hierarchy:

```
src/lua_exts/
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

- **Integration with handler scheme**: The `LuaExt` methods form the basis for the `AipFromLua` and `AipIntoLua` trait implementations (in `src/lua_exts/lua_traits.rs`), which are used to convert between Lua tables and Rust types in `aip.*` handler functions (see [`dev/specs/spec-handler-scheme.md`](spec-handler-scheme.md)).


## LuaJsonExt Trait (Lua ↔ JSON Conversion)

### When to use

Use `LuaJsonExt` whenever the application needs to convert between Lua values and JSON (e.g., for HTTP responses, configuration merging, or serialising Lua data to a wire format). Its custom conversion logic avoids pitfalls of the default `mlua` serde integration.

### Intent

Provide a set of extension methods for `mlua::Value` and `mlua::Table` that convert between `serde_json::Value` and `mlua::Value` with three key improvements over `mlua`'s built-in serde bridge:

1. **Null mapping** – `serde_json::Value::Null` is converted to `mlua::Value::NULL` (the crate's null sentinel) instead of opaque userdata, so that `LuaExt::x_is_null()` can uniformly recognise nulls.
2. **Array vs object heuristics** – When converting a Lua table to JSON, the trait inspects the keys; if the table has only contiguous 1..n integer keys, it emits a JSON array; otherwise it emits a JSON object with stringified keys.
3. **Nil‑to‑None semantics** – `x_to_json_value` returns `Result<Option<serde_json::Value>>`, allowing callers to distinguish a Lua `nil` (`Ok(None)`) from a JSON `null` (`Ok(Some(serde_json::Value::Null))`), which is impossible with the default conversion.

The trait has `LuaExt` as a supertrait, so all `LuaExt` query helpers (`x_get_string`, `x_as_list`, etc.) are available on any value that supports JSON conversion.

### Public API

#### Trait: `LuaJsonExt`

```rust
pub trait LuaJsonExt: LuaExt {
    fn x_from_json_value(lua: &Lua, val: serde_json::Value) -> crate::Result<Value>;
    fn x_from_json_values<I>(lua: &Lua, values: I) -> crate::Result<Value>
    where
        I: IntoIterator<Item = serde_json::Value>;
    fn x_to_json_value(&self) -> crate::Result<Option<serde_json::Value>>;
    fn x_to_json_values(&self) -> crate::Result<Option<Vec<serde_json::Value>>>;
}
```

**`x_from_json_value`** – Converts a single `serde_json::Value` into a `mlua::Value`. JSON `null` becomes `Value::NULL`, scalars become the corresponding Lua primitives, objects become Lua tables with string keys, and arrays become 1‑based Lua tables.

**`x_from_json_values`** – Converts an iterable of JSON values into a single Lua table with 1‑based integer keys, forming an array (list).

**`x_to_json_value`** – Converts the Lua value into a JSON value. Returns `Ok(None)` for `nil`, `Ok(Some(json))` for convertible types, and `Err` for unsupported types (functions, userdata, threads, etc.). Tables use the array/object heuristic.

**`x_to_json_values`** – If the value is a table, extracts its list elements (via `LuaExt::x_as_list`) and converts each element to JSON, returning `Ok(Some(vec))`. Returns `Ok(None)` for non‑table or `nil`.

### Code Design

The trait is implemented for both `mlua::Value` and `mlua::Table`. The `Table` implementations delegate to the `Value` implementations by wrapping the table in `Value::Table(self.clone())`. A private helper `convert_table` handles the array‑vs‑object inspection.

The implementation resides in `src/lua_exts/lua_json_ext.rs`. It uses `crate::Result<T>` for all conversions, returning descriptive errors on unsupported types.

### Design Considerations

- **Supertrait `LuaExt`** ensures that any value implementing `LuaJsonExt` also has all the `x_get_*` and `x_as_*` helpers, enabling fluid composition of key access and JSON conversion.
- **Null sentinel** – `Value::NULL` is a `LightUserData` sentinel recognised by `x_is_null`. This is different from both `nil` and JSON `null`, providing three states: absent (nil), null (sentinel), and a present value.
- **Array detection** requires two passes over the table entries to ensure no non‑sequential keys exist. This is O(n) and acceptable for typical request/response sizes.
- **JSON number conversion** uses `serde_json::Number::from_f64` and returns an error for non‑finite floats (`NaN`, `Infinity`), consistent with JSON constraints.
- **Future extensions** – additional helpers (e.g., `x_to_json_string`, bulk converters) can be layered on top of these primitives.
