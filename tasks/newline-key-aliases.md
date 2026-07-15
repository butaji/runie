# Newline key aliases

## Objective

Add `Ctrl+Enter` and `Alt+Enter` as aliases for inserting a newline in the chat input, matching the de-facto standard across terminal agents.

## Agent landscape finding

codex, gemini-cli, kimi-code, and goose all use `Ctrl+Enter`/`Alt+Enter` for newline while plain `Enter` submits.

## runie current state

Runie supports `Shift+Enter`, `Ctrl+J`, and `F3` for newline. `Enter` submits.

## Required runie changes

- Add `Ctrl+Enter` and `Alt+Enter` as additional newline events in the input keymap.
- Keep existing newline bindings for backward compatibility.
- Update the hint bar and `/hotkeys` to list the new bindings.

## Test scenarios

1. **Ctrl+Enter inserts newline**
   - Keys: `type `hello` press C-Enter type `world``
   - Assert: pane shows `hello` and `world` on separate lines before submission.

2. **Alt+Enter inserts newline**
   - Keys: `type `line1` press M-Enter type `line2``
   - Assert: both lines visible in input area.

3. **Plain Enter still submits**
   - Keys: `type `hello` press Enter`
   - Assert: message appears in transcript.

## Edge / negative cases

- Newline aliases work in the middle of input, not just at the end.
- Newline aliases do not open the command palette or trigger completions.

## Dependencies

- `input_composition`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
- [ ] Tests use `keys::` constants.
- [ ] `expect_text` uses robust regex alternations.
