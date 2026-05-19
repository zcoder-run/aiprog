# AIProg

Crate status: AIProg is in the early design and implementation phase.

AIProg is a Rust engine for moving AI systems from tool-call chaining to program execution.

Instead of asking an AI model to choose one tool at a time, AIProg asks it to write a small, constrained Lua program. The runtime executes that program through approved, Rust-backed, sandboxed APIs.

## Simple example

```lua
local page = aip.web.get("https://example.com")
local slimmed = aip.html.slim(page)
local summary = aip.genai.exec({
	model = "gemini-3.1-flash-lite",
	message = "Summarize this page:\n---\n" .. slimmed
})

return summary
```

An AI program does not need to call an AI model to be useful. It can use any available APIs to process, transform, and format data before returning results to the caller or host application.

## APIs

AIProg is designed around natural script functions.

- `aip.*` APIs are the batteries-included APIs, inspired by AIPack.
- Rust authors can add custom APIs or namespaces.
- Tools, MCPs, services, and host functions can be exposed as script functions that AI-authored programs orchestrate.

For example, an application might expose:

- `aip.file.*`
- `aip.git.*`
- `aip.web.*`
- `aip.llm.*`
- `zc.workspace.*`
- `myapp.deploy.*`

The AI does not need to reason over raw tool schemas. It writes a program against the APIs made available for the task.

## Positioning

AIProg asks the AI to write a program using available capabilities.

It moves AI systems from chaining tool calls to orchestrating workflows as code.
