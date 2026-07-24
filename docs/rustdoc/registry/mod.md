# Handler registry

The registry defines the Rust-to-Lua handler boundary.

Use [`AipRegistryBuilder`] to register handlers and build an immutable [`AipRegistry`]. A registry can be cloned cheaply, merged with another registry, or filtered with path patterns through [`AipRegistry::select`] and [`AipRegistry::exclude`].

## Handler types

A handler parameter type implements [`AipParams`], and an output type implements [`AipOutput`]. These traits combine Lua conversion, JSON schema generation, thread-safety, and static lifetime requirements.

Handlers return [`HandlerResult`], allowing handler-specific failures to be reported to Lua as runtime errors.

The [`AipHandler`] trait is implemented by types generated through the [`aip_handler`](crate::aip_handler) attribute macro. The macro captures handler metadata and schema information for documentation and registry introspection.

## Registration approaches

- Use [`AipRegistryBuilder::register_sync`] for a synchronous closure.
- Use [`AipRegistryBuilder::register_async`] for an asynchronous closure.
- Use [`AipRegistryBuilder::register_handler`] with an `#[aip_handler]` generated handler type.
- Use [`AipRegistryBuilder::add_module`] with an [`AipModule`](crate::AipModule) to compose a module of related handlers.

## Registry selection

[`RegistrySelectionOptions`] controls unmatched-pattern behavior. With [`UnmatchedPatternPolicy::Error`], selecting or excluding with a pattern that matches no handler returns [`RegistrySelectionError`]. This is useful when the configured script surface must be validated strictly.
