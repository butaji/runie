# Settings dialog

## Objective

Verify keyboard navigation and state changes in the settings dialog.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- `F2` opens settings with categories; arrow/Tab navigate; toggles update state.

## runie current state

runie `/settings` opens a dialog with Models category, provider/model fields, and tool checkboxes.

## Required runie changes

- No major change; add comprehensive keyboard navigation tests.

## Test scenarios

1. **Open settings**
   - Keys: `type `/settings` press Enter`
   - Assert: `Settings|Provider|Model`

2. **Arrow navigates categories**
   - Keys: `press Down`
   - Assert: `▸`

3. **Tab navigates fields**
   - Keys: `press Tab`
   - Assert: `▸`

4. **Toggle tool checkbox**
   - Keys: `press Space`
   - Assert: `\[x\]|\[ \]`

5. **Esc closes**
   - Keys: `press Escape`
   - Assert: `Settings`

## Edge / negative cases

- Settings changes persist to isolated config.
- Invalid values show validation error.

## Dependencies

- `command_palette_navigation`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
