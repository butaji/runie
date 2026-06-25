# Extract headless CLI helper module

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: collapse-headless-binaries-into-one-cli

## Description

`runie-print`, `runie-json`, and `runie-server` all share the same runtime setup boilerplate. Extract it into a shared module in `runie-agent`.

Shared boilerplate across all three:
```rust
use runie_agent::{run_headless_cli, HeadlessCliOptions};
use runie_core::permissions::build_sink;
use runie_core::message::ChatMessage;
```

Create `crates/runie-agent/src/headless_helper.rs` that provides:
1. A helper to build a system prompt with tools.
2. A helper to build the initial `ChatMessage` list from a user prompt.
3. A helper to construct `HeadlessCliOptions` with the common defaults (`execute_tools: true`, `max_tool_rounds: 5`).
4. Re-export `run_headless_cli` and `HeadlessCliOptions` for convenience.

After extraction, update `runie-print`, `runie-json`, and `runie-server` to import from `headless_helper`.

## Acceptance criteria

- [x] `crates/runie-agent/src/headless_helper.rs` created with shared helpers.
- [x] `runie-print` uses `headless_helper` for boilerplate.
- [x] `runie-json` uses `headless_helper` for boilerplate.
- [x] `runie-server` uses `headless_helper` for boilerplate.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `helper_builds_system_prompt` — `headless_helper::build_system_prompt()` returns a non-empty string.
- [x] `helper_builds_messages` — `headless_helper::build_messages("hello")` returns a vec with one user message.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `print_mode_still_works` — existing test in `runie-print/src/main.rs` still passes.

## Files touched

- `crates/runie-agent/src/headless_helper.rs` (new)
- `crates/runie-agent/src/lib.rs` (export headless_helper)
- `crates/runie-print/src/main.rs` (use headless_helper)
- `crates/runie-json/src/main.rs` (use headless_helper)
- `crates/runie-server/src/main.rs` (use headless_helper)

## Notes

This is a prerequisite for `collapse-headless-binaries-into-one-cli` but is also independently useful — it reduces duplication in the three headless binaries even if they remain separate crates.

The F34 audit finding says F34 (collapse into one CLI) "supersedes" F23 (this task), but the collapse task currently depends on this one. The relationship is: this task extracts the helper (a code quality win on its own); the collapse task then uses the helper when collapsing into one binary. Both tasks can be done independently; this one is the prerequisite.
