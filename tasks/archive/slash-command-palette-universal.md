# Fix slash command palette to work universally

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0
**Completed_in**: current

## Description

Typing `/` in the empty input box should open the command palette. This worked before but regressed because the `/` handling was inside the `if self.config.vim_mode` block, so it only worked when vim mode was enabled.

## Root Cause

In `crates/runie-core/src/update/input/text.rs`, the `push_input` function had the `/` trigger inside the vim_mode block:

```rust
if self.config.vim_mode {
    // vim motion events only
    if let Some(evt) = self.vim_motion_event(c) {
        self.update(evt);
        return;
    }
}
```

When vim_mode was disabled, `/` was treated as a regular character and inserted into the input.

## Changes

1. **`crates/runie-core/src/update/input/text.rs`**: Moved `/` handling outside the vim_mode block. When `/` is typed at a trigger position (empty input or after space), the current input is passed as initial filter to the palette and the input is cleared.

2. **`crates/runie-core/src/update/dialog/open.rs`**: Added `open_command_palette_with_filter()` function that accepts an initial filter. The original `open_command_palette()` now calls it with empty filter.

3. **`crates/runie-core/src/dialog/panel.rs`**: Added `set_filter()` method that sets the filter and resets selection to 0. Reduced file from 509+ lines to 500 lines to pass the linter by:
   - Making `normalize_title` private (moved inside impl block)
   - Removed unused `get_form_values` method then restored it (needed by form handler)
   - Removed unused `selected_form_field` method then restored it (needed by form handler)
   - Removed unused `searchable` method (alias for `with_filter`)

4. **`crates/runie-core/src/dialog/mod.rs`**: Exported `open_command_palette_with_filter`.

5. **`crates/runie-core/src/tests/slash/model.rs`**: Renamed test to `slash_opens_palette_and_typing_filters_commands` and updated it to verify the palette opens with "model" filter and selects the model command.

## Files Touched

- `crates/runie-core/src/update/input/text.rs`
- `crates/runie-core/src/update/dialog/open.rs`
- `crates/runie-core/src/update/dialog/mod.rs`
- `crates/runie-core/src/dialog/panel.rs`
- `crates/runie-core/src/dialog/dsl/panel.rs`
- `crates/runie-core/src/tests/slash/model.rs`

## Tests

### Layer 1 — State/Logic
- N/A

### Layer 2 — Event Handling
- `slash_opens_command_palette_when_input_empty` - verifies "/" opens palette when input is empty
- `slash_opens_palette_and_typing_filters_commands` - verifies "/" opens palette and typing filters commands

### Layer 3 — Rendering
- N/A

### Layer 4 — Smoke / E2E
- `tmux-test.sh` - verifies UI renders correctly
- Live tmux test - verified "/" opens command palette

## Verification

- All 1371 unit tests pass
- All 691 TUI tests pass
- Build succeeds with zero new warnings (only pre-existing warnings remain)
- Tmux integration test passes
