# r2-turn-complete-conditional

Turn completed should be shown in the feed only if >1 think/run actions in a turn. If =1, no need to show.

## Problem

The "Turn completed in X.Xs" indicator was showing even for trivial single-action turns (e.g., just a thought, or just one tool call). This is visual noise — users don't need a "turn complete" indicator when the turn was trivial.

## Solution

In `ui/transform.rs`, `collect_entries` already counts how many `Thought` and `Tool` messages belong to each turn (`action_counts`). TurnComplete is **skipped** in the feed when that count is ≤ 1.

## Tests

### Layer 1: State/Logic

| Test | Description |
|------|-------------|
| `single_thought_hides_turn_complete` | Turn with only 1 Thought → no TurnComplete in feed |
| `single_tool_hides_turn_complete` | Turn with only 1 Tool → no TurnComplete in feed |
| `two_thoughts_shows_turn_complete` | Turn with 2 Thoughts → TurnComplete visible |
| `two_tools_shows_turn_complete` | Turn with 2 Tools → TurnComplete visible |
| `mixed_thought_tool_shows_turn_complete` | Turn with Thought + Tool → TurnComplete visible |
| `zero_actions_hides_turn_complete` | Turn with no actions (just response) → TurnComplete hidden |
| `second_turn_not_affected` | Two turns; turn 1 has 2 actions, turn 2 has 1 → only turn 2 hides |

### Layer 2: Event Handling (flow test)
| Test | Description |
|------|-------------|
| `full_flow_single_action_hides` | Complete flow: thinking → thought done → turn complete → done → TurnComplete not in feed |
| `full_flow_multi_action_shows` | Complete flow: thinking → thought done → tool → tool end → turn complete → done → TurnComplete in feed |
