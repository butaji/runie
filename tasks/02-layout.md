# Step 02: Layout

**Status:** pending
**Depends on:** 01

## Goal
A pure function that splits the terminal `Rect` into scrollback / prompt / status regions.

## Changes
- `crates/runie-tui/src/layout.rs`:
  - `pub fn chat_layout(area: Rect) -> ChatLayout { scrollback, prompt, status }`.
  - `ChatLayout` struct with three `Rect`s (status = 1 row, prompt = 3 rows, scrollback = remainder).
  - `pub const STATUS_HEIGHT: u16 = 1;`
  - `pub const PROMPT_HEIGHT: u16 = 3;`

## Verification
- Unit test in `layout.rs`: given a 24x80 area, scrollback is 20 rows, prompt is 3, status is 1.

## Notes
- Pure function; no ratatui state. Just `Rect` arithmetic.