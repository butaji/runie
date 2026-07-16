# Tool permission dialog

## Objective

Verify that tool/edit permission dialogs offer four choices and are fully keyboard navigable.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Grok edit dialog: `1 Yes, and don't ask again`, `2 Yes, allow all edits during this session`, `3 Yes`, `4 No, reject`.
- Footer: `1/4:select | Ctrl+O:yolo | Ctrl+C:cancel`.

## runie current state

runie has a three-option dialog: Allow / Deny / Always Allow.

## Required runie changes

- Add fourth option `This session` between `Always` and `Once`.
- Relabel to: `Always`, `This session`, `Once`, `Deny`.
- Support number keys 1-4 as direct selectors.
- Persist `Always` across sessions, `This session` until session end, `Once` single call, `Deny` as rejection.

## Test scenarios

1. **Dialog opens**
   - Keys: `type `list files` press Enter`
   - Assert: `Permission Required`

2. **Four options visible**
   - Keys: `capture pane`
   - Assert: `Always|This session|Once|Deny`

3. **Number 1 activates Always**
   - Keys: `press 1`
   - Assert: `Cargo\.toml|src/`

4. **Number 4 rejects**
   - Keys: `re-open dialog press 4`
   - Assert: `Permission denied`

5. **Arrow navigation**
   - Keys: `press Down Down Enter`
   - Assert: `tool output|denied`

6. **Tab navigation**
   - Keys: `press Tab Tab Enter`
   - Assert: `tool output|denied`

7. **Always skips dialog**
   - Keys: `trigger tool again`
   - Assert: `Cargo\.toml|src/` (no new Permission Required dialog)

## Edge / negative cases

- Deny with accelerator `y` still works if kept.
- Dialog is non-closable with Esc until a choice is made.

## Dependencies

- `command_palette_navigation`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
