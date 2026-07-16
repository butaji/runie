# Undo and redo commands

## Objective

Add `/undo` and `/redo` slash commands to revert and re-apply the most recent assistant turn, including any file edits it made.

## Agent landscape finding

gptme, opencode, and kimi-code support reverting the last turn and restoring it. This is a high-trust affordance when the model makes an unwanted change.

## runie current state

Runie has `/approve` and `/reject` for pending file edits, but no transcript-level undo once a turn is complete.

## Required runie changes

- Add `/undo` slash command that removes the last user+assistant turn pair from the transcript and reverts any file edits from that turn.
- Add `/redo` slash command that re-applies a previously undone turn.
- Scope to one turn at a time; multi-turn undo remains a future session-tree feature.
- Persist undo history for the current session only.

## Test scenarios

1. **Undo removes last turn**
   - Keys: `type `say hello` press Enter .wait_for_response type `/undo` press Enter`
   - Assert: assistant response disappears from transcript.

2. **Undo reverts file edit**
   - Keys: trigger a mock edit, allow it, then `/undo`.
   - Assert: edited file content is restored to original.

3. **Redo restores undone turn**
   - Keys: `/undo`, then `/redo`.
   - Assert: assistant response reappears and file edit is re-applied.

4. **Redo disabled when nothing to redo**
   - Keys: `/redo` without prior `/undo`.
   - Assert: inline error or no-op; app remains stable.

## Edge / negative cases

- `/undo` with only one turn in transcript clears the transcript.
- `/undo` during an active turn aborts first, then undoes.
- File edits outside the project are not affected by undo.

## Dependencies

- `tool_permissions`
- `turn_lifecycle`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` or `AppTest::mock_with_fixture(...)`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
- [ ] Tests use `keys::` constants.
