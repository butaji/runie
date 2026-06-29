# Rescope terminal capability task to current module layout

**Status**: todo
**Milestone**: R6
**Category": TUI / Rendering
**Priority": P2

**Depends on**: none
**Blocks**: none

## Description

`simplify-terminal-capability-detection.md` still references deleted files (`term_caps.rs`, `app.rs`). The current code lives in `crates/runie-tui/src/terminal/caps/mod.rs` and `detect.rs`. Update the task to target the current module and decide whether to keep or delete the remaining brand/multiplexer/mouse/clipboard/focus/unicode heuristics.

## Acceptance Criteria

- [ ] Update `simplify-terminal-capability-detection.md` with current file paths and ACs.
- [ ] Decide fate of remaining heuristics (brand table, multiplexer detection, clipboard OSC 52, focus tracking).
- [ ] If a heuristic duplicates a crate (`supports-color`, `supports-hyperlinks`, `crossterm`), remove it.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `term_caps_from_current_module` — `TermCaps` is built from current module.

### Layer 3 — Rendering
- [ ] `render_uses_term_caps` — ANSI escapes respect capability snapshot.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `tasks/simplify-terminal-capability-detection.md`
- `crates/runie-tui/src/terminal/caps/mod.rs`
- `crates/runie-tui/src/terminal/caps/detect.rs`
- `crates/runie-tui/src/terminal/caps/tests.rs`

## Notes

- This is a meta-task to keep planning docs accurate; no implementation code is required unless scope expands.
