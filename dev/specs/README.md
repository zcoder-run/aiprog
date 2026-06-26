# Specs Directory

This directory contains internal specification documents for the project.

Last reviewed: 2026-06-26.

The `spec-*.md` files (e.g., `spec-handler-scheme.md`, `spec-lua-ext.md`, `spec-webc.md`) serve as internal specifications, code design documents, and private best practices. They are not intended for external publication.

## Purpose

- **Internal specification**: Define the intended behavior, API surface, and design rationale for various components.
- **Code design**: Outline architecture, module design, and implementation decisions.
- **Private best practices**: Document conventions, patterns, and guidelines specific to this codebase.

These documents should be kept up-to-date with the codebase and consulted when making changes or adding new features.

## Specs

- **`spec-handler-scheme.md`**: Describes the framework for writing `aip.*` Lua module handlers in Rust, including error handling, parameter extraction via `AipFromLua`, and the single-result output pattern using `AipOutput` and `AipIntoLua`.
- **`spec-lua-ext.md`**: Covers the `LuaExt` trait and related utilities for working with Lua tables and values, used by handler implementations for parameter extraction.
- **`spec-webc.md`**: Documents the `WebClient` backend for making HTTP requests, the `web.get` and `web.post` Lua functions, and the separation between the Rust backend and the `aip.web` Lua frontend.
- **`README.md`**: This file – describes the purpose and contents of the specs directory.
