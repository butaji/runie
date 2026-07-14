# Compact command

## Objective

Add a `/compact` slash command that summarizes older conversation turns to reduce context-window pressure, with a preview and confirmation.

## Agent landscape finding

goose, gptme, and gemini-cli have `/compact` or `/summarize` commands. They help keep long sessions within context limits.

## runie current state

Runie does not have context compaction.

## Required runie changes

- Add `/compact [optional instruction]` slash command.
- Generate a summary of older turns using the model.
- Show a preview of which turns will be compacted and the resulting summary.
- Require confirmation before replacing the compacted turns with the summary.
- Hint the user when context usage crosses a configurable threshold.

## Test scenarios

1. **Compact previews summary**
   - Keys: have a multi-turn conversation, type `/compact` press Enter
   - Assert: preview dialog shows summary and turns to be compacted.

2. **Confirm compaction**
   - Keys: in preview, confirm.
   - Assert: compacted turns collapse into a summary block; transcript remains coherent.

3. **Cancel compaction**
   - Keys: in preview, press `Esc`.
   - Assert: transcript unchanged.

## Edge / negative cases

- `/compact` with fewer than two turns shows "Nothing to compact".
- Compaction cannot be undone; warn the user.

## Dependencies

- `turn_lifecycle`
- `status-command`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` or replay fixtures.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
