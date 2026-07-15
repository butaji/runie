# Startup and onboarding

## Objective

Verify first-run behavior and idle quit.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Welcome screen with New worktree / Resume session / Changelog / Quit.
- Project-directory prompt before chat.

## runie current state

runie onboarding shows a provider picker that is blocking until a provider is connected.

## Required runie changes

- No change; ensure keyboard navigation and close-guard behavior is covered.

## Test scenarios

1. **Onboarding provider picker**
   - Keys: `AppTest::onboarding() start`
   - Assert: `Choose a provider`

2. **Picker navigable**
   - Keys: `press Down`
   - Assert: `▸`

3. **Esc does not close**
   - Keys: `press Escape`
   - Assert: `Choose a provider`

4. **Complete onboarding**
   - Keys: `select mock enter key select echo`
   - Assert: `mock/echo|Type a message`

5. **Ctrl+Q from idle**
   - Keys: `AppTest::mock() press C-q`
   - Assert: `process exited`

## Edge / negative cases

- Text quit commands (`quit`, `exit`, `:q`) do **not** close the onboarding
  provider picker until a provider and model are selected; `Ctrl+Q` is the only
  allowed quit path from idle onboarding.
- Onboarding with invalid config shows validation.

## Dependencies

- None

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
