## ■ Step - Add AipHandlerMeta and handler_meta() to AipHandler trait
      status: not_started
## ✅ Step - Add AipHandlerMeta and handler_meta() to AipHandler trait
      status: done
time-created: 2026-06-28 17:11:54
   time-done: 2026-06-28 17:14:15

Add the infrastructure for the proc-macro to emit metadata via a `handler_meta()`

- Build and test to confirm no regressions.

### Work done

- Added `AipHandlerMeta` struct in `src/registry/handler_traits.rs` with `description` and `title` fields,
  re-exported as `crate::registry::AipHandlerMeta`.
- Added `handler_meta() -> AipHandlerMeta` default method to the `AipHandler` trait (returns empty).
- Added `pub(crate) use` re-export in `mod.rs`.
- Existing `handler_desc()` and `handler_title()` methods are unchanged.
- `cargo build`, `cargo test`, `cargo clippy` pass.

## ■ Step - Refactor proc-macro to implement AipHandler on function type and generate __aiprog_meta_ helper
