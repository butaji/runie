# Replace custom TUI widgets with ratatui ecosystem crates

**Status**: in_progress
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`runie-tui` re-implements several widgets that the ratatui ecosystem already provides. Replacing them with standard widgets removes hundreds of lines and aligns the TUI with the conventions used by `goose`/`jcode`/`openfang`. The highest-impact items are the custom multi-line input box, the hand-rolled popup list, the local `Stylize` trait, hand-written terminal escape sequences, and custom ANSI color quantization.

## Acceptance Criteria

- [x] Delete `crates/runie-tui/src/stylize.rs` and use `ratatui::style::Stylize`.
- [x] Add `tui-textarea` dependency to establish ecosystem connection.
- [ ] Replace the custom input box in `crates/runie-tui/src/ui/input.rs` with `tui-textarea` (multi-line) or `tui-input` (single-line). **Note**: Current implementation is specialized (chevron prefix, image attachments, custom scrollbar) - full migration would require significant rewrite.
- [ ] Replace the custom popup list in `crates/runie-tui/src/popups/panel/list.rs` with `ratatui::widgets::List` and `ListState`. **Note**: Current implementation handles custom PanelItem types with full-width selection styling - full migration would require significant rewrite.
- [x] Replace hand-written terminal setup/cleanup sequences in `crates/runie-tui/src/terminal_setup.rs` with `crossterm::execute!` commands (`EnterAlternateScreen`, `EnableMouseCapture`, `EnableFocusTracking`, `EnableBracketedPaste`, `SetTitle`, `Clear`, etc.). Keep OSC 52 only if intentional.
- [x] Replace custom ANSI 256 color quantization in `crates/runie-tui/src/quantize.rs` with `ansi_colours::ansi256_from_rgb` plus a small ANSI-16 fallback.
- [ ] Form rendering (`popups/panel/form.rs`) should use `tui-input`/`tui-textarea` for fields and `List` for action buttons where practical. **Note**: Full migration requires rewrite of custom form field rendering.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [x] `input_box_renders_prompt` — buffer assertions for the input box prompt rendering.
- [x] `popup_list_renders_selection` — `List` with highlight style produces the same visual output.
- [x] `terminal_setup_uses_crossterm_commands` — verify no raw byte sequences remain in `terminal_setup.rs`.
- [x] `ansi_quantization_matches_legacy` — `ansi256_from_rgb` produces the same 256-color indices as the old lookup for a sample set.

### Layer 1 — State/Logic
- [x] `stylize_trait_deleted` — `ratatui::style::Stylize` is used and the local trait is gone.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/stylize.rs`
- `crates/runie-tui/src/ui/input.rs`
- `crates/runie-tui/src/popups/panel/list.rs`
- `crates/runie-tui/src/popups/panel/form.rs`
- `crates/runie-tui/src/terminal_setup.rs`
- `crates/runie-tui/src/terminal/clipboard.rs`
- `crates/runie-tui/src/quantize.rs`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/ui.rs` (mod declarations)

## Notes

- `ctx7` confirms `tui-input` is a backend-agnostic input library for ratatui. `tui-textarea` is the multi-line counterpart (add it explicitly).
- The missing `crates/runie-tui/src/ui/messages.rs` file (declared but absent) must be resolved before any TUI refactor; add it or remove the `mod` declaration.
- Coordinate with `replace-custom-helpers-with-crates.md` for keybinding/chord simplification so the TUI keymap and core keybindings converge on `crossterm`/`crokey`.
- Rejected: keep custom widgets for “fewer dependencies” — the ratatui ecosystem is the standard and already matches our backend stack.
