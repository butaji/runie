# Block-Level Copy & Clipboard Fallback

**Status**: todo
**Milestone**: R4
**Category**: TUI / Clipboard
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie already has OSC 52 clipboard wiring for `/copy` and `/copy-last`. Add
vim-style block-level copy (`y` / `Y`) and a platform fallback when OSC 52 is
unavailable.

## Acceptance Criteria

- [ ] In vim nav mode, `y` copies the raw plain text of the selected block.
- [ ] In vim nav mode, `Y` copies block metadata (timestamp, model, tool name).
- [ ] User/assistant blocks copy their text content.
- [ ] Tool blocks copy `command + result` as plain text.
- [ ] Thinking blocks copy the summary; if expanded, copy full reasoning.
- [ ] When `caps.clipboard` is false, fall back to `pbcopy` on macOS,
  `xclip`/`wl-copy` on Linux, and `clip` on Windows.
- [ ] Emit a transient success/error message after copy.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn copy_block_text_emits_clipboard_event() {
    let mut state = AppState::with_assistant_message("hello");
    state.select_first_block();
    state.update(Event::CopySelectedBlock);
    // Assert CopyToClipboard("hello") event emitted.
}

#[test]
fn copy_tool_block_includes_command_and_result() {
    let mut state = AppState::with_tool_done("list .", "file1\nfile2");
    state.select_first_block();
    state.update(Event::CopySelectedBlock);
    // Assert clipboard text contains both command and result.
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn y_in_vim_nav_copies_block() {
    let mut state = AppState::with_assistant_message("hello");
    state.vim_nav_mode = true;
    state.update(key_event('y'));
    // Assert copy event.
}
```

## Files touched

- `crates/runie-core/src/update/input_nav.rs`
- `crates/runie-core/src/event/input.rs` (add CopySelectedBlock event)
- `crates/runie-term/src/effects/clipboard.rs` (add fallback)
- `crates/runie-term/src/terminal/clipboard.rs` (platform fallbacks)

## Out of scope

- Mouse right-click copy menu.
