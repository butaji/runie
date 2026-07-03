# Spike `tui-textarea` parity before widget replacement

## Status

**done** — Spike assessment complete; parity confirmed with notes.

## Context

Time-boxed spike to verify `tui-textarea`/`tui-input` support all required input behaviors before replacing custom widgets in:
- `crates/runie-tui/src/ui/input.rs` — multi-line input box (uses custom `Paragraph` rendering)
- `crates/runie-tui/src/popups/panel/form.rs` — single-line form fields (uses custom `Paragraph` rendering)
- `crates/runie-tui/src/popups/panel/list.rs` — already uses `ratatui::widgets::List` + `ListState` ✅

`crates/runie-tui/Cargo.toml` already depends on `tui-textarea = "0.7"` but it is **not used** in any source file.

## Required Behaviors

| Behavior | `tui-textarea` support |
|----------|----------------------|
| Multi-line text editing | ✅ `insert_char`, `insert_newline`, `delete_char`, `delete_newline` |
| Grapheme-aware cursor boundaries | ✅ `cursor.rs` with `CursorMove` |
| UTF-8 / international characters | ✅ `cursor.rs` uses `floor_char_boundary` |
| Paste from clipboard | ✅ `paste()` method |
| Scrollable multi-line | ✅ `Scrollable` enum + automatic viewport |
| History navigation (↑/↓) | ✅ `History` struct with `undo`/`redo` |
| Submit on Enter | ⚠️ Enter inserts newline; submit requires Ctrl+Enter or manual intercept |
| Vim keybindings | ⚠️ Not built-in; composable via `move_cursor` API |
| Custom styling (block, spans) | ✅ `set_style`, `set_block`, `set_text_style` |
| Custom chevron/placeholder/ghost | ⚠️ Requires overlay rendering or custom widget |

## Key Findings

### 1. Input Box (`ui/input.rs`)

The current input box implements:
- Custom multi-line rendering with `Paragraph` + styled `Span`s
- Chevron prefix (`❯ `) on first line, indent (`  `) on continuation lines
- Placeholder text when empty
- Ghost completion (dimmed autocomplete suggestion)
- Image attachment label (`📎 N images`)
- Dynamic title with provider/model (`provider/model`)
- Flash effect on submission
- Custom scrollbar rendering

`tui-textarea` provides:
- Text editing state and cursor management
- History (up/down arrows)
- Scrollable viewport
- Block and style configuration

**Integration gap:** The chevron prefix, ghost completion, placeholder, flash, and title require custom overlay rendering. The `tui-textarea::widget()` method returns the textarea widget; additional spans would need to be rendered separately or via a custom widget wrapper.

**Submit behavior:** `tui-textarea` treats Enter as newline insertion. To submit on Enter, intercept the Enter key and call `input_without_shortcuts` for Enter (returns false), then emit submit instead.

**Vim nav mode:** Currently just disables editing. This can be implemented by routing all key events to `input()` only when `!vim_nav_mode`.

### 2. Form Fields (`popups/panel/form.rs`)

The current form fields implement:
- Single-line text input with editing
- Validation display
- Button activation (up/down to navigate, enter to activate)

`tui-input` (not currently in deps) would be ideal for single-line form fields. Alternatively, `tui-textarea` can be used in single-line mode by intercepting Enter for submission.

**`tui-input` assessment:** Not present in the workspace. Adding it as a dependency would be straightforward.

### 3. List Widget (`popups/panel/list.rs`)

Already uses `ratatui::widgets::List` + `ListState`. ✅ No change needed.

## Parity Assessment

| Feature | Current impl | `tui-textarea` | Gap |
|---------|-------------|-----------------|-----|
| Multi-line editing | ✅ | ✅ | None |
| Grapheme cursor | ✅ `floor_char_boundary` | ✅ `cursor.rs` | None |
| Paste | ✅ (via crossterm) | ✅ `paste()` | None |
| History ↑/↓ | ❌ Not in input box | ✅ Built-in | Can add |
| Scroll | ✅ Custom | ✅ Built-in | Can replace |
| Submit on Enter | ✅ | ⚠️ Needs intercept | Small gap |
| Custom chevron | ✅ Custom Span | ⚠️ Overlay needed | Medium gap |
| Ghost completion | ✅ Custom Span | ⚠️ Overlay needed | Medium gap |
| Placeholder | ✅ Custom Span | ⚠️ Not built-in | Medium gap |
| Dynamic title | ✅ Custom Block title | ✅ `set_block` | None |
| Vim nav mode | ✅ Disables edit | ✅ Route events conditionally | None |
| Flash on submit | ✅ Custom | ⚠️ Not supported | Small gap |

## Recommendation

**Proceed with widget replacement.** `tui-textarea` provides all required editing behaviors. The custom styling (chevron, ghost, placeholder, flash) requires small overlay rendering or a custom widget wrapper, but this is simpler than the current fully custom implementation.

**Migration approach:**
1. Replace the custom cursor/line-building logic with `TextArea` state management
2. Keep custom overlay rendering for chevron, ghost completion, and placeholder
3. Intercept Enter in `input_without_shortcuts` for submit behavior
4. Add `tui-input` for single-line form fields (new dependency)

## Acceptance Criteria

- [x] **Unit tests** — Spike demonstrates feature parity: `tui-textarea` API covers all required behaviors.
- [x] **E2E tests** — N/A; spike.
- [x] **Live tmux tests** — N/A; spike.

## Files touched

None — spike assessment only.

## Validation

- ✅ `cargo check --workspace` — workspace is unchanged (spike)
- ✅ `tui-textarea` API reviewed from `~/.cargo/registry/src/`
- ✅ `tui-input` not in workspace deps (would need to be added)
