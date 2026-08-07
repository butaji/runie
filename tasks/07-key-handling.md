# Step 07: Key handling

**Status:** implemented; remaining Grok-only overlays stay out of Pi scope (2026-08-07)
**Depends on:** 06

## Goal
Translate `crossterm::event::KeyEvent` into `Action` events the app loop dispatches.

## Changes
- `crates/runie-tui/src/key.rs`:
  - `pub enum Action { Submit(String), Abort, Quit, Clear, FocusPrompt }`.
  - `pub fn map_key(key: KeyEvent, prompt_text: &str, prompt_dirty: bool) -> Vec<Action>`:
    - `Enter` → if prompt non-empty: `Submit(text)`; else `FocusPrompt`.
    - `Ctrl+C` → if streaming: `Abort`; else `Quit`.
    - `Ctrl+D` → `Quit`.
    - `Ctrl+L` → `Clear` (scrollback).
    - `Esc` → `Clear` if prompt empty.

## Verification
- Unit test per mapping; table-driven test covering all branches.

## Notes
- Pure function; no IO. Easy to test exhaustively.
