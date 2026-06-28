# Centralize built-in tool names

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

The canonical list of built-in tool names already exists in `crates/runie-core/src/tool/mod.rs` as `BUILTIN_TOOL_NAMES`, and `crates/runie-agent/src/tool/mod.rs` re-exports it. However, several consumers still hard-code the same names locally, and the system-prompt/tool-schema-enricher paths build parallel lists. This task finishes the centralization.

Still duplicated or parallel:

- `crates/runie-agent/src/tool_runner.rs:46-57` (`dispatch_tool`) and `:62-68` (`is_known_tool`)
- `crates/runie-agent/src/headless/mod.rs:314-324` (`build_tool_registry`)
- `crates/runie-agent/src/turn/mod.rs:245-258` (`build_tool_registry` with read-only filtering)
- `crates/runie-agent/src/inspector.rs:87-98` (`dispatch_tool`)
- `crates/runie-agent/src/tests/tools.rs:15-25` (`dispatch_tool` test helper)
- `crates/runie-agent/src/turn/mod.rs:265-267` builds the system-prompt tool list as a literal string
- `crates/runie-core/src/harness_skills/tool_schema_enricher.rs:38-53` hard-codes example tools and omits `search`/`find_definitions`

## Acceptance Criteria

- [ ] Every location above references `runie_core::tool::BUILTIN_TOOL_NAMES` / `is_builtin_tool` instead of repeating literal names.
- [ ] The system-prompt tool list is generated from the canonical list (respecting read-only flags if needed).
- [ ] The schema-enricher examples cover every `BUILTIN_TOOL_NAMES` entry or are removed in favor of the canonical list.
- [ ] Read-only filtering in `turn/mod.rs` remains correct.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `builtin_tools_registered_once` — verifies the list is defined once and every consumer resolves to the same set.
- [ ] `schema_enricher_covers_all_builtin_tools` — asserts the enricher examples include every canonical tool name.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_still_dispatches_builtin_tools` — runs a provider-replay turn that exercises built-in tools and confirms dispatch still works after centralization.

## Files touched

- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-agent/src/inspector.rs`
- `crates/runie-agent/src/tests/tools.rs`
- `crates/runie-core/src/harness_skills/tool_schema_enricher.rs`

## Notes

- The canonical list already exists; this task is about switching consumers to it.
- This is an independent, high-Pareto task: small, safe, and removes a duplication hotspot.
- Out of scope: changing tool schemas, MCP boundary, or skill-hook logic.
