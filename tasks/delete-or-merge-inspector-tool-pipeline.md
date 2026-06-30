# Delete or merge the inspector tool pipeline

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: centralize-built-in-tool-names
**Blocks**: none

## Description

The `ToolPipeline` + `Inspector` trait in `runie-agent/src/inspector.rs` was dead code: exported from `runie-agent` but never imported or used anywhere in the codebase. The `runie inspect` CLI command (`crates/runie-cli/src/inspect.rs`) is unrelated — it prints runtime configuration and is actively used.

**Action taken**: Deleted `crates/runie-agent/src/inspector.rs` and removed `pub mod inspector` from `runie-agent/src/lib.rs`. Removed the stale exemption for `inspector.rs` in `runie-core/build.rs`.

## Acceptance Criteria

- [x] `inspect` either is removed or delegates to the shared tool execution path. — `inspect` in CLI (`runie inspect`) is the config-inspection command; it was never related to the tool pipeline. The dead tool pipeline is deleted.
- [x] No separate inspector-specific rendering module remains unless it is a thin wrapper. — N/A.
- [x] Tool-call display uses one formatter across TUI, CLI, and inspector. — N/A.
- [x] `cargo test --workspace` succeeds after the change. — Verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — Verified (no new warnings).

## Tests

### Layer 1 — State/Logic
- [x] N/A — the module was dead code; deletion doesn't affect any test logic.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-agent/src/inspector.rs` — **deleted**
- `crates/runie-agent/src/lib.rs` — removed `pub mod inspector`
- `crates/runie-core/build.rs` — removed stale exemption for `inspector.rs`

## Notes

- `runie inspect` (config introspection) and the tool-inspector pipeline were always two separate things.
- The `ToolPipeline` was designed as middleware around tool calls but was never wired into the agent tool loop.
- The `runie-cli/src/inspect.rs` file is unaffected and remains active.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
