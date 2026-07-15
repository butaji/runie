# Session management

## Objective

Verify new/save/load/resume/list session commands and their keyboard navigation.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Commands: `/new`, `/save`, `/load`, `/resume`, `/sessions`, `/rename`.

## runie current state

runie has `/new`, `/save`, `/load`, `/resume`, `/sessions`, `/name`.

## Required runie changes

- Add `/rename` alias if missing; ensure session list is keyboard navigable.

## Test scenarios

1. **New session**
   - Keys: `type `/new` press Enter`
   - Assert: `New session|Session`

2. **Save session**
   - Keys: `type `/save` press Enter`
   - Assert: `saved|Session`

3. **List sessions**
   - Keys: `type `/sessions` press Enter`
   - Assert: `saved sessions`

4. **Resume session**
   - Keys: `type `/resume` press Enter`
   - Assert: `mock/echo`

5. **Rename session**
   - Keys: `type `/rename test` press Enter`
   - Assert: `test`

## Edge / negative cases

- New session clears chat but keeps provider config.
- Resume nonexistent session shows error.

## Dependencies

- `core_mock_loop`
- `command_palette_navigation`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
