# Keyboard shortcuts overlay

## Objective

Verify that the keyboard-shortcuts overlay is discoverable, searchable, and dismissible.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Grok: `Ctrl+X` opens a searchable overlay with categories (Essentials, Input, Conversation Navigation, ...).
- Overlay footer: `↑/↓ nav | f filter | e/Space/→ expand | ← collapse | Enter details | / search | Esc close`.

## runie current state

runie has `/hotkeys` which opens a scrollable list of 43 bindings. It is searchable/filterable but not yet grouped into expandable categories.

## Required runie changes

- No UI redesign required. Test the existing overlay open/close/filter behavior.
- Optionally note category grouping as a future improvement.

## Test scenarios

1. **Open with /hotkeys**
   - Keys: `type `/hotkeys` and press Enter`
   - Assert: `Keyboard Shortcuts|43 bindings`

2. **Open with Ctrl+P then select hotkeys**
   - Keys: `press Ctrl+P, type `hotkeys`, press Enter`
   - Assert: `Keyboard Shortcuts`

3. **Filter bindings**
   - Keys: `type `quit``
   - Assert: `ctrl\+q|ForceQuit`

4. **Close with Esc**
   - Keys: `press Escape`
   - Assert: `Type a message|mock/echo`

## Edge / negative cases

- Overlay does not block Ctrl+Q quit.
- Empty filter shows all bindings.

## Dependencies

- None

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
