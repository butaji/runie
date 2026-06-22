# Adopt Snapshot Testing with `insta`

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Add snapshot testing infrastructure using `insta` crate for TUI output validation. Currently Layer 3 tests use TestBackend + Buffer assertions; insta provides:

- Human-readable snapshot files (`.snap` files in `snapshots/` directory)
- `insta::assert_snapshot!()` macro for easy assertions
- `cargo insta review` for reviewing snapshot changes
- `cargo insta test` with `--accept` for auto-updating

Reference: `~/Code/agents/codex-rs/core/src/tools/handlers/` and `tests/` for snapshot test patterns.

## Acceptance Criteria

- [x] `insta` added as test dependency.
- [x] Snapshot tests for key UI components (chat messages, tool outputs, diff rendering).
- [x] `cargo insta review` workflow documented.
- [x] `cargo test --workspace` succeeds with new snapshots.

## Tests

### Layer 1 — State/Logic
N/A.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [x] `snapshot_chat_message_renders_correctly` — chat widget output.
- [x] `snapshot_tool_output_renders_correctly` — tool result display.
- [x] `snapshot_diff_renders_correctly` — diff viewer output.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `Cargo.toml` — add `insta` to workspace dev-dependencies
- `crates/runie-tui/` — add snapshot tests

## Notes

Snapshots complement (not replace) TestBackend assertions. Use snapshots for output shape, TestBackend for exact pixel counts.
