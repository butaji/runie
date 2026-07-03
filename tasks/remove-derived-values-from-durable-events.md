# Remove derived values from durable events

## Status

`done`

## Description

Removed `duration_secs` from `DurableCoreEvent::ToolResult` in the JSONL session store. During live turns, `duration_secs` is computed from timing data and displayed in the UI. During replay (when timing data isn't available), it defaults to `0.0`.

## Changes

1. **`DurableCoreEvent::ToolResult`** — Removed `duration_secs` field from the struct. The field was previously stored in JSONL with `#[serde(default)]`.

2. **`try_from_event`** — `Event::ToolEnd` → `DurableCoreEvent::ToolResult` no longer copies `duration_secs`.

3. **`try_from DurableCoreEvent`** — `ToolResult` → `Event::ToolEnd` reconstructs with `duration_secs: 0.0` (timing data not available during replay).

4. **Tests** — Updated to reflect that `duration_secs` is not preserved through the durable layer.

## Acceptance criteria

1. ✅ **Unit tests** — Durable event schema has no derived fields.
2. ✅ **E2E tests** — Replay works (duration shows as 0.0 for replayed sessions).
3. ✅ **Live tmux tests** — Save/resume session works (live duration shown in session).

## Tests

- `durable_from_tool_end` — verifies `duration_secs` NOT in durable form
- `tool_result_roundtrip` — verifies `duration_secs: 0.0` after round-trip
- `durable_tool_result_roundtrips_through_json` — JSON roundtrip works
- `event_from_tool_result` — event reconstruction works without duration_secs
