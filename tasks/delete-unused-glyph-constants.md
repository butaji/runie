# Delete unused glyph constants from `runie-tui`

**Status**: done
**Milestone**: R6
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/glyphs.rs` does not exist. The glyph constants it was supposed to contain have already been cleaned up (or were moved to `theme/glyph.rs` in a prior refactor). The only remaining reference is a comment in `ui.rs`.

## Acceptance Criteria

- [x] Identify which glyphs are actually used. — **N/A**: the file does not exist.
- [x] Move used glyphs to `theme/glyph.rs`. — **N/A**.
- [x] Delete `crates/runie-tui/src/glyphs.rs`. — **Already done**: file does not exist.
- [x] Update imports. — **N/A**.
- [x] `cargo test --workspace` succeeds after the change. — Already verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — Already verified.

## Tests

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- None — the file was already removed before this task was authored.

## Notes

- Glyphs used in the TUI are defined in `crates/runie-tui/src/theme/glyph.rs` (which exists).
- The only reference to "glyphs" in the codebase is a design-system comment in `ui.rs`.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
