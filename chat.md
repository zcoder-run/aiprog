
## Request: Implement step: Add AipHandlerMeta and handler_meta() to AipHandler trait

### Summary

Added `AipHandlerMeta` struct with `description` and `title` fields in `handler_traits.rs`, re-exported as `crate::registry::AipHandlerMeta`. Extended the `AipHandler` trait with a default `handler_meta()` method returning empty metadata. The existing `handler_desc()` and `handler_title()` methods remain unchanged. Build and tests pass.

### Next steps

Proceed with the next plan step to refactor the proc-macro to implement `AipHandler` on the function type and generate `__aiprog_meta_` helper.
