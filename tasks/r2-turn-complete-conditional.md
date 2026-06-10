# r2-turn-complete-conditional

Turn completed should be shown in the feed only if >1 think/run actions in a turn. If =1, no need to show.

## Problem

The "Turn completed in X.Xs" indicator was showing even for trivial single-action turns (e.g., just a thought, or just one tool call). This is visual noise — users don't need a "turn complete" indicator when the turn was trivial.

## Solution

In `ui/transform.rs`, `collect_entries` already counts how many `Thought` and `Tool` messages belong to each turn (`action_counts`). TurnComplete is **skipped** in the feed when that count is ≤ 1.

## Bugs Found & Fixed

### Bug 1: `move_turn_complete_to_end()` moved ANY TurnComplete
**Symptom:** With multiple turns, finishing turn 2 would move turn 1's TurnComplete to the end, causing it to appear after turn 2's content.
**Fix:** Pass `id` parameter to `move_turn_complete_to_end` so it only moves the TurnComplete for the current turn.
**Test:** `turn_complete_order_preserved_across_multiple_turns`

### Bug 2: `ensure_turn_complete_last()` moved ANY TurnComplete
**Symptom:** Every agent event (including turn 2's events) would move turn 1's TurnComplete to the end and bump its timestamp, causing it to leapfrog turn 2's content.
**Fix:** Only move TurnComplete when its `id` matches `current_request_id`, falling back to `last_assistant_index`'s message id for delayed chunks after `AgentDone`.
**Test:** `turn_complete_order_preserved_across_multiple_turns` + existing delayed-chunk tests

## Tests

### Layer 1: State/Logic

| Test | Description |
|------|-------------|
| `single_thought_hides_turn_complete` | Turn with only 1 Thought → no TurnComplete in feed |
| `tool_only_hides_turn_complete` | Turn with only 1 Tool → no TurnComplete in feed |
| `tool_plus_thought_shows_turn_complete` | Turn with 1 Thought + 1 Tool → TurnComplete visible |
| `two_thoughts_shows_turn_complete` | Turn with 2 Thoughts → TurnComplete visible |
| `two_tools_shows_turn_complete` | Turn with 2 Tools → TurnComplete visible |
| `mixed_thought_tool_shows_turn_complete` | Turn with Thought + Tool → TurnComplete visible |
| `zero_actions_hides_turn_complete` | Turn with no actions (just response) → TurnComplete hidden |
| `second_turn_independent_action_count` | Two turns; turn 1 has 2 actions, turn 2 has 1 → only turn 2 hides |
| `three_mixed_actions_shows_turn_complete` | Turn with 3 actions → TurnComplete visible |
| `turn_complete_still_in_session_when_hidden` | TurnComplete hidden from feed but still in session.messages |
| `turn_complete_order_preserved_across_multiple_turns` | Turn 1's TurnComplete stays before turn 2's content |

### Layer 2: Event Handling (existing + new)
All existing `turn_complete_order` tests continue to pass (12 tests).
