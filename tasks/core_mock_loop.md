# Core mock provider loop

## Objective

Verify that mock providers start and respond deterministically.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Basic ask/response with working credentials.

## runie current state

runie-tests already has mock echo and list_dir tests.

## Required runie changes

- Extend coverage for status/turn state, not just response text.

## Test scenarios

1. **Mock env var starts**
   - Keys: `AppTest::mock() start`
   - Assert: `mock/echo|Type a message`

2. **Mock CLI flag starts**
   - Keys: `AppTest::mock_cli_flag() start`
   - Assert: `mock/echo`

3. **Echo responds**
   - Keys: `type `hello` press Enter`
   - Assert: `→\s*hello`

4. **List dir fixture**
   - Keys: `AppTest::mock_with_fixture('list_dir') type `list files` allow`
   - Assert: `Cargo\.toml|src/`

## Edge / negative cases

- Mock provider survives rapid restarts.
- Unknown fixture falls back gracefully.

## Dependencies

- None

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
