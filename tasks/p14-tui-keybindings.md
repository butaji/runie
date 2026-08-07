# p14 — TUI: keybinding parity (Shift+Tab, Ctrl+x, Ctrl+L model selector, history, file-search, multiline)

Status: complete for the Pi-feature subset; Grok-only file-viewer semantics are documented separately (2026-08-08)

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
8. **Ctrl+L**: open Pi's model selector (`pi/packages/coding-agent/src/core/keybindings.ts:85`). Grok's file-search line-viewer binding is a Grok-only surface and is not assigned this Pi-compatible key in Runie.

## State machine / variants

Input-mode state machine:
```
normal --Shift+Tab--> mode2 ; mode2 --Shift+Tab--> normal
normal --Shift/Alt+Enter--> multiline ; multiline --Enter--> submit
normal --Enter--> submit ; normal --"/"--> history_search ; normal --Up(empty)--> history_browse
history_browse --Up/Down--> navigate ; --Enter--> select ; --Esc--> cancel
normal --"@"--> file_search ; file_search --Tab/Enter--> accept ; --Esc--> cancel
normal --Ctrl+L--> model_selector ; model_selector --Enter--> commit ; --Esc--> cancel
normal --Ctrl+C(non-empty)--> clear ; --Ctrl+C(empty)--> cancel_run
```
Key event variants to handle: `Enter`, `Shift+Enter`, `Alt+Enter`, `Tab`, `Shift+Tab`, `Up`, `Down`, `Esc`, `Ctrl+C`, `Ctrl+D`, `Ctrl+Q`, `Ctrl+X`, `Ctrl+L`, `:`, `/`.

## Acceptance

- Unit tests for each key path (multiline, history nav, clear-vs-cancel, quit keys, mode cycle).
- Snapshot/key handling tests in `runie-tui` green; `cargo test --workspace` green.

## Progress

- **In progress (2026-08-05):** Shift/Alt-Enter multiline input, prompt history,
  Shift+Tab, Ctrl+X, Ctrl+L, quit chords, and clear-vs-abort routing are
  covered. The binary now routes clear, abort, and quit actions through the
  shared key mapper. Remaining work is wiring the full app-level mode/file-
  search state machine and rendering the shortcut surface from the owned app
  state; the mode and shortcut transitions are now runtime-wired, the prompt
  mode transition has a unit test, and the shortcut state renders a
  deterministic panel in both initial and steady-state frames.
- **Ctrl+L Pi correction (2026-08-08):** Runie follows Pi's `app.model.select`
  binding. File search remains prompt-local and does not claim Grok's global
  line-viewer binding.
- **File-search entry (2026-08-05):** Ctrl+L now enters an owned prompt
  `FileSearch` mode and renders a `file search` caption; Esc returns to normal
  input. Result selection and the line-viewer handoff remain to be implemented.
- **YAML coverage (2026-08-05):** Added `visual-file-search.yaml`; the YAML
  runner accepts a `Ctrl+L` step and checks the resulting prompt chrome.
- **Candidate interaction (2026-08-05):** File-search mode now lists up to
  five visible current-directory candidates, supports Up/Down selection, and
  accepts the selected candidate with Tab/Enter.
- **Line viewer (2026-08-05):** An accepted file can now be reopened through
  Ctrl+L into a bounded 20-line in-prompt viewer; Esc returns to normal input.
  Added a regression test for the handoff and viewer exit.
- The YAML file-search fixture now covers the complete entry → accept → viewer
  transition.

## Completion

All specified key paths are now runtime-wired and covered: multiline input,
history navigation/search, clear-vs-abort, quit chords, mode cycling, shortcut
panel, file-search candidate selection, bounded viewer handoff, and Esc exit.
Focused TUI tests, the YAML fixture suite, strict clippy, and local formatting
checks pass.
