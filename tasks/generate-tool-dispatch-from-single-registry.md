# Generate tool dispatch from a single registry

## Status

**partial** — Removed unused `ALL_TOOL_NAMES` duplicate; dispatch still hardcoded.

## Context

`dispatch_tool` and `BUILTIN_TOOL_NAMES` are hand-maintained lists. Generate both from a single registry using a macro or `inventory`/`linkme`.

## Changes Made

1. **Removed `ALL_TOOL_NAMES`** from `tool_registry.rs` — it was an unused duplicate of `BUILTIN_TOOL_NAMES` from `runie_core`.
2. **Updated module docs** to clarify that tool names are defined in `runie_core::tool::BUILTIN_TOOL_NAMES`.

## Remaining Work

The dispatch in `tool_runner.rs` is still hardcoded. To fully implement this task:

1. Create a tool registry trait or macro that:
   - Registers each tool with its name and implementation
   - Generates the name list automatically
   - Generates the dispatch match automatically

2. Options:
   - Use `inventory` crate to register tools at compile time
   - Use a declarative macro to generate dispatch
   - Use a custom derive macro with attributes

## Acceptance criteria

- [ ] Unit tests — Adding a tool requires changing only one source of truth; name list and dispatch stay in sync.
- [ ] E2E tests — All built-in tools still execute correctly in mock-provider replay.
- [ ] Live tmux tests — Run bash, read, grep, find, and edit tools in tmux.

## Tests

### Unit tests
- Registry contains all built-in tools; dispatch routes to the right implementation.

### E2E tests
- Multi-tool replay turn uses every built-in tool.

### Live tmux tests
- Submit a prompt that triggers read/grep/edit/bash tools.
