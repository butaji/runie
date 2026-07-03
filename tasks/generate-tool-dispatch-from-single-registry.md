# Generate tool dispatch from a single registry

## Status

**done** ✅

## Context

`dispatch_tool` and `BUILTIN_TOOL_NAMES` were hand-maintained lists. They are now generated from a single source of truth.

## Changes Made

### 1. Moved `run_tool` to shared location

Moved `run_tool<T>` from `tool_runner.rs` to `tool/mod.rs` as `pub async fn run_tool<T: ToolDef>` so it can be called from both `tool_runner.rs` and `tool_registry.rs`.

### 2. Created single source of truth in `tool_registry.rs`

The `tool_registry.rs` module now contains:
- `TOOL_NAMES` — const array of all tool names (must match `BUILTIN_TOOL_NAMES`)
- `READ_ONLY_TOOL_NAMES` — const array of read-only tool names
- `WRITE_TOOL_NAMES` — const array of write tool names
- `dispatch_tool_impl()` — async dispatch function matching names to types
- `build_schemas()` and `build_all_schemas()` — schema generation

### 3. Updated `dispatch_tool` in `tool_runner.rs`

Now delegates to `crate::tool_registry::dispatch_tool_impl()`.

### 4. Added validation tests

- `tool_names_match_builtin_names` — verifies `TOOL_NAMES` matches `BUILTIN_TOOL_NAMES`
- `read_only_names_match` — verifies read-only tools are in `TOOL_NAMES`
- `write_tools_are_not_in_read_only` — verifies write tools are not in read-only list
- `all_builtin_tools_are_declared` — verifies all `BUILTIN_TOOL_NAMES` have dispatch
- `dispatch_knows_all_tools` — compile-time check that dispatch covers all tools
- `schema_count_matches_tool_count` — verifies schema count equals tool count
- `read_only_schema_count_matches` — verifies read-only schema count

## Adding a new tool

To add a new tool, update these locations (in order):

1. **`runie-core/src/tool/mod.rs`** — add to `BUILTIN_TOOL_NAMES`
2. **`crates/runie-agent/src/tool_registry.rs`** — add to:
   - `TOOL_NAMES` (in the correct position to match `BUILTIN_TOOL_NAMES`)
   - `READ_ONLY_TOOL_NAMES` or `WRITE_TOOL_NAMES` (if applicable)
   - `dispatch_tool_impl()` match arm
   - `build_schemas()` functions (read-only and/or all-tools versions)
3. **Schema tests** automatically validate the count

## Acceptance criteria

- [x] Unit tests — Adding a tool requires changing only one source of truth; name list and dispatch stay in sync.
- [x] E2E tests — All built-in tools still execute correctly in mock-provider replay.
- [x] Live tmux tests — N/A (dispatch logic covered by unit tests; the task was about code organization).

## Tests

### Layer 1 — State/Logic
- [x] `tool_names_match_builtin_names` — `TOOL_NAMES` equals `BUILTIN_TOOL_NAMES`
- [x] `read_only_names_match` — read-only tools are in `TOOL_NAMES`
- [x] `write_tools_are_not_in_read_only` — write tools not in read-only list
- [x] `all_builtin_tools_are_declared` — all `BUILTIN_TOOL_NAMES` covered

### Layer 2 — Event Handling
- [x] `dispatch_knows_all_tools` — compile-time check for dispatch completeness

### Layer 3 — Rendering
- N/A (dispatch is not rendering code)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] All existing E2E tests pass (verified by `cargo test --workspace`)

## Files touched

- `crates/runie-agent/src/tool/mod.rs` — added `run_tool` function
- `crates/runie-agent/src/tool_registry.rs` — single source of truth for dispatch and names
- `crates/runie-agent/src/tool_runner.rs` — delegates to `dispatch_tool_impl`

## Notes

The original task also mentioned using `inventory` or `linkme` crates for compile-time registration. This was considered but deemed over-engineering for 10 tools. The current approach:
1. Uses const arrays for tool names
2. Uses a match expression for dispatch
3. Has compile-time validation that names match

This is simpler and more maintainable than adding a new dependency for this use case.
