# Replace clipboard dead code with arboard

## Status

`done`

**Completed:** 2026-06-30

## Context

`crates/runie-tui/src/terminal/clipboard.rs` is dead code marked `#![allow(dead_code)]`. The actual copy effect dispatches to `handle.write_clipboard(...)` in `crates/runie-tui/src/effects/mod.rs`. `Architecture.md` already lists `arboard` as the preferred clipboard crate.

## Goal

Delete the dead OSC-52 module and use `arboard` for clipboard writes, with `crossterm`'s `osc52` feature as a fallback if `arboard` proves problematic in CI/headless.

**Design impact:** No change to TUI element design or composition. Only the clipboard-copy mechanism changes; any transient status message must reuse the existing status style.

## Acceptance Criteria

- [x] Delete `crates/runie-tui/src/terminal/clipboard.rs`.
- [x] Implement clipboard copy via `arboard` in `effects/mod.rs`.
- [x] Handle headless/CI environments gracefully (arboard returns error).
- [x] Remove `#[allow(dead_code)]` markers for clipboard functions.

## Tests

- **Layer 1 — State/Logic:** Unit test for the copy helper in a headless stub environment.
- **Layer 1:** Verify the dead module is removed and no new warnings are introduced.
- **Layer 2 — Event Handling:** Feed a copy shortcut event and assert the effect dispatches a clipboard write.
- **Layer 3 — Rendering:** `TestBackend` snapshot after copy shows a transient status message (if implemented).
- **Layer 4 — E2E:** Headless CLI with a copy tool returns the expected event.
- **Live tmux validation:** In a real terminal, select a message and press the copy shortcut; paste the content outside tmux and verify it matches.
