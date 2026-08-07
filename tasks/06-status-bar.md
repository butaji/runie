# Step 06: Status bar

**Status:** implemented; Grok metric/effect expansion remains (2026-08-07)
**Depends on:** 05

## Goal
A 1-row status widget that reflects the loop's current state.

## Changes
- `crates/runie-tui/src/widgets/status.rs`:
  - `pub enum Status { Ready, Thinking, Streaming, Aborted, Error(String) }`.
  - `pub struct StatusBar { state: Status }`.
  - `pub fn set(&mut self, s: Status)`.
  - `pub fn render(&self, area: Rect, buf: &mut Buffer)`: renders as `> <state>` with color (green=ready, yellow=thinking, blue=streaming, red=error).

## Verification
- Unit test: each `Status` variant renders a distinct label + color.

## Notes
- 1 row tall; no scrolling needed.
