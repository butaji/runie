# Status bar

## Objective

Verify that runie's single global status line is as complete as Grok's.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Top line shows cwd, model, token usage `11K / 512K`.
- During turn: `Waiting for response` with timer and `[stop]`.
- After turn: `Turn completed in Xs`.

## runie current state

runie shows a status line with model and token usage, but lacks explicit state/timing text.

## Required runie changes

- Add state text: idle, waiting, completed, error.
- Add timing information after turn completion.
- Keep the single-line layout above the input box.

## Test scenarios

1. **Idle state**
   - Keys: `start app`
   - Assert: `Type a message|mock/echo|0/128k`

2. **Waiting state**
   - Keys: `type `hi` press Enter`
   - Assert: `Waiting|Thinking|⠋|⠙`

3. **Completed state**
   - Keys: `wait`
   - Assert: `completed|done`

4. **Token usage updates**
   - Keys: `wait`
   - Assert: `[0-9]+/[0-9]+[kK]?`

5. **Model indicator**
   - Keys: `capture pane`
   - Assert: `mock/echo`

## Edge / negative cases

- Error/auth state shown as error text without crashing.
- Status line survives dialog open/close.

## Dependencies

- `turn_lifecycle`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
