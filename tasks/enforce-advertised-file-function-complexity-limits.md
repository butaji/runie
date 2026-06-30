# Enforce advertised file/function/complexity limits

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: none

## Description

`AGENTS.md` claims the build enforces 500-line files, 40-line functions, and complexity ≤10, but `crates/runie-core/build.rs` only checks `AppState` field access and manifest checksums. Many production functions already violate the advertised limits.

## Root Cause

The linter was either removed or never fully implemented, while the documentation still claims strict enforcement.

## Acceptance Criteria

- [ ] Either update `AGENTS.md` to match reality or implement the advertised checks in `build.rs`/CI.
- [ ] If enforcing, fix or split the worst existing violations so the build passes.
- [ ] If relaxing, document the rationale and remove the false claim.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` has no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `build_script_enforces_limits` — a test fixture file/function that exceeds limits fails the build check.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — build/CI concern.

## Files touched

- `AGENTS.md`
- `crates/runie-core/build.rs`
- Oversized production files identified in the review.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Major known violations:
  - `crates/runie-core/src/actors/input/messages.rs:83` — `apply_to` ≈ 208 lines, complexity 30
  - `crates/runie-core/src/markdown/blocks.rs:189` — `push_text` 119 lines, complexity 15
  - `crates/runie-core/src/diff/mod.rs:129` — `parse` 281 lines, complexity 36
  - `crates/runie-core/src/bash_safety.rs:21` — `check_destructive_tokens` 194 lines, complexity 81
  - `crates/runie-tui/src/message/support.rs:15` — `render_thought_marker` 264 lines, complexity 36
  - `crates/runie-tui/src/diff.rs:18` — `render_canonical_diff` 241 lines, complexity 20
