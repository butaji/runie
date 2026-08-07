# Step 05: Prompt widget

**Status:** implemented; Grok interaction refinements remain in p14/p21
**Depends on:** 04

## Goal
A prompt widget wrapping `tui-textarea`: render in a `Rect`, accept keyboard input, return submitted text on Enter.

## Changes
- `crates/runie-tui/src/widgets/prompt.rs`:
  - `pub struct PromptWidget { inner: TextArea<'static>, placeholder: String, focused: bool }`.
  - `pub fn new() -> Self` with placeholder "> ".
  - `pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome`: `Enter` returns `Submitted(text)`, `Backspace`/`Char`/etc. return `Edited`, anything else `Ignored`.
  - `pub fn render(&self, area: Rect, buf: &mut Buffer)`.
  - `pub fn clear(&mut self)`.
  - `pub fn set_focused(&mut self, on: bool)`.

## Verification
- Unit test: type "hi", press Enter, assert `Submitted("hi")`.
- Render test: empty prompt renders the placeholder.

## Notes
- tui-textarea lives behind our wrapper so future swaps (e.g. to a custom widget) are local.
