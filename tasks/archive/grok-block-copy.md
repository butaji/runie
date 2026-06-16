# Block-Level Copy & Clipboard Fallback

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1
**Completed in**: current

**Depends on**: (none)
**Blocks**: (none)

## Description

Added vim-style block-level copy (`y` / `Y`) and a platform clipboard fallback
when OSC 52 is unavailable.

## Changes Made

### 1. `CopySelectedBlock` and `CopyBlockMetadata` events
- Added `CopySelectedBlock` and `CopyBlockMetadata` variants to `DialogEvent` in
  `event/dialog_display.rs`
- Updated `variant_name()` and `Display` impl for both
- Added both to `names.rs` as bindable events

### 2. Text extraction from selected post
Added to `model/state.rs`:
- `copy_selected_post_text(&self)` — extracts plain text from the selected post:
  user/agent messages → content; tool-running → `name args`; tool-done → `name args\noutput`
- `copy_selected_post_metadata(&self)` — extracts short metadata string:
  `role timestamp` for messages; `name done in Xs at Ys` for tool blocks
- `element_text(elem)` helper — plain text per element kind
- `element_metadata(elem)` helper — metadata string per element kind

### 3. Vim `y` / `Y` wired in `input_nav.rs`
- `try_vim_nav_motion('y')` → emits `DialogEvent::CopySelectedBlock`, exits nav mode
- `try_vim_nav_motion('Y')` → emits `DialogEvent::CopyBlockMetadata`, exits nav mode

### 4. TUI effect dispatch
Updated `effects/mod.rs`:
- Added `CopySelectedBlock { text: String }` and `CopyBlockMetadata { text: String }` variants to `EffectCommand`
- `try_from_event` extracts text via `state.copy_selected_post_text()` / `copy_selected_post_metadata()`
- Both dispatch to `clipboard::copy_to_clipboard(text, caps)`

### 5. Platform clipboard fallback
Updated `effects/clipboard.rs`:
- `copy_to_clipboard` tries OSC 52 first, then falls back to `platform_copy`
- macOS: `pbcopy` (stdin piped)
- Linux (unix): tries `wl-copy` then `xclip` (stdin piped)
- Windows: `cmd /C echo text | clip`
- All failures are silent (fallback is best-effort)

## Acceptance Criteria

- [x] In vim nav mode, `y` copies the raw plain text of the selected block
- [x] In vim nav mode, `Y` copies block metadata (timestamp, model, tool name)
- [x] User/assistant blocks copy their text content
- [x] Tool blocks copy `command + result` as plain text
- [x] Thinking blocks copy the summary (via `ThoughtSummary` element)
- [x] When `caps.clipboard` is false, falls back to `pbcopy` (macOS), `wl-copy`/`xclip` (Linux), `clip` (Windows)
- [x] `y` / `Y` always exits vim nav mode (even with no selection)
- [x] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State / Logic (in `tests/copy.rs`)
- `copy_selected_post_text_user_message` — y on user message returns content
- `copy_selected_post_text_agent_message` — y on agent message returns content
- `copy_selected_post_text_tool_done` — y on tool post returns command+output
- `copy_selected_post_metadata_returns_timestamp` — Y returns timestamp metadata
- `copy_selected_post_text_no_selection_returns_none` — no post selected → None
- `copy_selected_post_text_empty_post_returns_none` — spacer-only post → None

### Layer 2 — Event Handling (in `tests/vim_mode.rs`)
- `y_in_vim_nav_copies_block_and_exits_nav` — y emits event and exits nav
- `Y_in_vim_nav_copies_metadata_and_exits_nav` — Y emits event and exits nav
- `y_on_empty_selection_does_not_crash` — y with no selection exits nav gracefully

## Files touched

- `crates/runie-core/src/event/dialog_display.rs` (CopySelectedBlock, CopyBlockMetadata)
- `crates/runie-core/src/event/names.rs` (bindable entries)
- `crates/runie-core/src/model/state.rs` (copy_selected_post_text, copy_selected_post_metadata, helpers)
- `crates/runie-core/src/update/input_nav.rs` (vim y/Y handlers)
- `crates/runie-tui/src/effects/mod.rs` (EffectCommand variants, dispatch)
- `crates/runie-tui/src/effects/clipboard.rs` (platform fallback)
- `crates/runie-core/src/tests/copy.rs` (Layer 1 tests)
- `crates/runie-core/src/tests/vim_mode.rs` (Layer 2 tests)
