# Turn lifecycle

## Objective

Verify the full user-message → streaming-response → turn-complete flow.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- User message appears immediately with timestamp.
- Response streams with `◆ Thought for Xs` blocks.
- Status bar shows `Waiting for response` and a stop button.
- Ends with `Turn completed in Xs`.

## runie current state

runie mock echo returns a response but does not show explicit 'Turn completed' timing or streaming state labels.

## Required runie changes

- Show turn state in status bar: idle / waiting / completed.
- Display a turn-complete marker or timing after the response finishes.

## Test scenarios

1. **User message rendered**
   - Keys: `type `hi` press Enter`
   - Assert: `❯ hi`

2. **Response appears**
   - Keys: `wait`
   - Assert: `→ hi|hi`

3. **Turn complete marker**
   - Keys: `wait`
   - Assert: `Turn completed|Done`

4. **Cancel turn**
   - Keys: `type `long` press Enter press C-c`
   - Assert: `stopped|cancelled`

5. **Follow-up after turn**
   - Keys: `type `again` press Enter`
   - Assert: `again`

## Edge / negative cases

- Rapid consecutive submits queue correctly.
- Cancelled turn does not corrupt subsequent turns.

## Dependencies

- `core_mock_loop`
- `input_composition`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
