# Dedupe truncate Function

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`truncate` function (14 lines) exists in both `agents.rs` and runie-tui. Use `textwrap::shorten` or extract to shared utils.

## Acceptance Criteria

- [ ] Add `truncate(s: &str, n: usize) -> String` to shared utils
- [ ] Replace both copies
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] `test_truncate_short_string` — strings <= n returned unchanged
- [ ] `test_truncate_long_string` — strings > n truncated with ellipsis

### Layer 2 — Event Handling
- [ ] Agent list rendering tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] N/A

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/agents.rs`
- `crates/runie-tui/src/` (find where duplicate exists)

## Notes

`textwrap::shorten` would be the cleanest solution since textwrap is already a dependency.
