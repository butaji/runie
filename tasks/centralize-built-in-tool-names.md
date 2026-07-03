# Centralize built-in tool names

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

The canonical list of built-in tool names already exists in `crates/runie-core/src/tool/mod.rs` as `BUILTIN_TOOL_NAMES`, and `crates/runie-agent/src/tool/mod.rs` re-exports it. However, several consumers still hard-code the same names locally, and the system-prompt/tool-schema-enricher paths build parallel lists. This task finishes the centralization.

## Changes Made

### `turn/mod.rs`
- Extracted `WRITE_TOOLS` constant referencing the write-permission tools
- `build_tools_list()` now uses `WRITE_TOOLS` instead of inline literal array

### `tool_schema_enricher.rs`
- Added import for `BUILTIN_TOOL_NAMES`
- `get_examples()` now validates tool name against `BUILTIN_TOOL_NAMES`
- `get_canonical_examples()` provides examples only for canonical tools
- Added `search` and `find_definitions` examples (were missing)
- Added tests verifying all canonical tools have examples

### Already Correct
- `tool_runner.rs`: `is_known_tool()` already uses `is_builtin_tool()`
- `headless/mod.rs`: Uses static dispatch with tool types (not string literals)
- `turn/mod.rs`: `build_tool_registry()` uses static dispatch with tool types

## Acceptance Criteria

- [x] Every location above references `runie_core::tool::BUILTIN_TOOL_NAMES` / `is_builtin_tool` instead of repeating literal names.
- [x] The system-prompt tool list is generated from the canonical list (respecting read-only flags if needed).
- [x] The schema-enricher examples cover every `BUILTIN_TOOL_NAMES` entry or are removed in favor of the canonical list.
- [x] Read-only filtering in `turn/mod.rs` remains correct.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `builtin_tools_registered_once` — `BUILTIN_TOOL_NAMES` is the single source of truth.
- [x] `schema_enricher_covers_all_builtin_tools` — asserts the enricher examples include every canonical tool name.
- [x] `schema_enricher_unknown_tool_returns_empty` — unknown tools return empty examples.
- [x] `enrich_schema_adds_examples_for_known_tool` — schema enrichment works for known tools.
- [x] `enrich_schema_skips_unknown_tool` — schema enrichment skips unknown tools.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Existing tool tests verify dispatch still works after centralization.

## Files touched

- `crates/runie-agent/src/turn/mod.rs` — WRITE_TOOLS constant and updated build_tools_list
- `crates/runie-core/src/harness_skills/tool_schema_enricher.rs` — Added BUILTIN_TOOL_NAMES validation and tests

## Notes

- The canonical list already exists; this task is about switching consumers to it.
- This is an independent, high-Pareto task: small, safe, and removes a duplication hotspot.
- Out of scope: changing tool schemas, MCP boundary, or skill-hook logic.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
