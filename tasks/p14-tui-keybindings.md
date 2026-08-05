# p14 — TUI: keybinding parity (Shift+Tab, Ctrl+x, Ctrl+L, history, Tab, file-search, multiline)

**Parity target:** grok-build pager keyboard surface.

## Grok reference

`~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/views/prompt_widget/mod.rs`
- Bare Enter **submits**; multiline via **Shift/Alt-Enter** (line 483-484: "Handles text editing, multiline (Shift/Alt-Enter), submit (Enter), and Ctrl-C (clear if non-empty)").
- **Ctrl-C** clears a non-empty prompt (line 484); does NOT quit.
- Esc / Tab / Ctrl-D are **focus/quit** management, handled by the caller, not the prompt widget (line 487-490).
- Footer hints: `Enter`, `Shift+Tab`, `Ctrl+x` (from p13).
- `~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/views/welcome/mod.rs:48-53`: quit hints are `ctrl+d` and `ctrl+q`.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/key.rs` + `widgets/prompt.rs` + `app.rs`
- Enter submits, Ctrl+C cancels/clears, Esc clears, Ctrl+D quits (per the old TUI plan 07-key-handling).

## Adapt to runie

1. **Multiline**: Shift/Alt-Enter inserts a newline; bare Enter submits. Add a multiline indicator to the prompt chrome (see p15).
2. **Ctrl+C**: clear the current prompt when non-empty; when the prompt is empty, act as cancel/interrupt the running agent.
3. **Ctrl+D / Ctrl+Q**: quit (both). Esc clears the prompt (does not quit).
4. **Shift+Tab**: cycle input mode (e.g. normal ↔ a second mode) — at minimum render the mode hint; wire to a mode toggle if the app has one.
5. **Ctrl+X**: a shortcut (e.g. open a shortcut-help / menu). If the minimal app has no target, reserve the key and show the hint.
6. **History browse**: Up on an empty prompt opens prompt history; `/history` search mode (grok `prompt_widget/mod.rs:499-500`). Implement prompt history ring + up/down navigation.
7. **File-search dropdown + Tab completion**: optional; if implemented, gate behind a feature flag and document the scope (grok uses a file-search dropdown driven by Tab).
8. **Ctrl+L**: open the selected file-search result in a line viewer (grok `prompt_widget/mod.rs:489`).

## State machine / variants

Input-mode state machine:
```
normal --Shift+Tab--> mode2 ; mode2 --Shift+Tab--> normal
normal --Shift/Alt+Enter--> multiline ; multiline --Enter--> submit
normal --Enter--> submit ; normal --"/"--> history_search ; normal --Up(empty)--> history_browse
history_browse --Up/Down--> navigate ; --Enter--> select ; --Esc--> cancel
normal --"/>"/Ctrl+L--> file_search ; file_search --Tab/Enter--> accept ; --Esc--> cancel
normal --Ctrl+C(non-empty)--> clear ; --Ctrl+C(empty)--> cancel_run
```
Key event variants to handle: `Enter`, `Shift+Enter`, `Alt+Enter`, `Tab`, `Shift+Tab`, `Up`, `Down`, `Esc`, `Ctrl+C`, `Ctrl+D`, `Ctrl+Q`, `Ctrl+X`, `Ctrl+L`, `:`, `/`.

## Acceptance

- Unit tests for each key path (multiline, history nav, clear-vs-cancel, quit keys, mode cycle).
- Snapshot/key handling tests in `runie-tui` green; `cargo test --workspace` green.