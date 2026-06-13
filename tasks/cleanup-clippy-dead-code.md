# Clean Up Clippy Warnings for Dead and Unused Code

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Description

`cargo clippy --workspace` currently reports warnings that indicate dead
or unused code:

- `runie-term/src/terminal/clipboard.rs`: `copy_to_clipboard`,
  `copy_to_primary`, `set_terminal_title`, `set_cursor_color`,
  `clear_screen`, `cursor_home`, `show_cursor`, `hide_cursor` are public
  but unused.
- `runie-term/src/terminal_setup.rs`: `enable_mouse`, `disable_mouse`
  are public but unused.
- `runie-core/src/state.rs`: `InputState.input_history` field is never
  read.
- `runie-core/src/commands/dsl/builder.rs`: `PanelItem` is imported but
  unused.
- `runie-core/src/commands/agents_manager.rs`: `AgentProfile` is
  imported but unused.
- `runie-core/src/update/tab_complete.rs`: `find_prefix_file_matches`
  and `suffix_after_prefix` are never used.
- `runie-core/src/event.rs`: `EVENT_NAMES` complex tuple type triggers
  `type_complexity`.
- `runie-core/src/model.rs`: `model_selector_items` returns a complex
  5-tuple `Arc` type.
- `runie-provider/src/mock.rs`: `String::from_utf8_lossy` after slicing
  a string (`sliced_string_as_bytes`).
- `runie-tui/src/ui.rs`: `repeat().take()` can be `repeat_n()`.
- `runie-provider/src/openai.rs`: unused `Context` import.

## Acceptance Criteria

- [ ] All listed unused/dead code is deleted, wired, or gated behind a
  feature flag.
- [ ] Complex tuple types get named type aliases (e.g.
  `type ModelSelectorItem = (String, String, String, bool, bool)`).
- [ ] `cargo clippy --workspace` produces zero warnings.
- [ ] `cargo fmt --check` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo clippy --workspace` passes with no warnings.
- [ ] `cargo fmt --check` passes.

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib keybindings` passes.

### Layer 3 — Rendering
- [ ] `cargo test -p runie-tui --lib` passes.

### Layer 4 — Smoke
- [ ] `./dev.sh` runs end-to-end.

## Notes

**Why zero warnings matters:**
Warnings train developers to ignore compiler output, which hides real
bugs. The workspace should stay warning-free.

**Out of scope:**
- Behavioral changes (only deletions/aliases/style fixes).

## Verification

```bash
cargo clippy --workspace 2>&1 | grep -E "warning|error" | wc -l
# Expected: 0

cargo fmt --check
```
