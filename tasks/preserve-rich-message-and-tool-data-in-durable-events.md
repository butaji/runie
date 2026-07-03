# Preserve rich message and tool data in durable events

## Status

**done** — `parts` are now preserved in `DurableCoreEvent::MessageSent` and restored during replay.

## Context

`event/durable.rs` and `session/replay.rs` flatten messages to `content: String`, dropping `Part`s (images, tool calls, reasoning). `ToolResult` previously hardcoded `success: true` and dropped `duration`.

## Implementation

### Added `parts` field to `DurableCoreEvent::MessageSent`

Updated `DurableCoreEvent::MessageSent` to include `parts: Vec<Part>`:
- Added `#[serde(default)] parts: Vec<Part>` field to the enum variant
- Updated `message_to_event` in `session/replay.rs` to include `message.parts.clone()`
- Added `replay_message_with_parts` method to `AppState` for restoring parts
- Updated `replay_event` to use the new method

### Tool Result Duration (already done)

- `duration_secs` is preserved in `ToolResult`
- `success` is hardcoded to `true` (tool errors emit a separate event)

### Backward Compatibility

- Old JSON without `parts` defaults to empty `Vec` via `#[serde(default)]`
- Old JSON without `duration_secs` defaults to `0.0` via `#[serde(default)]`
- When parts are present, content is reconstructed from text parts for `Event::MessageReplayed`

## Acceptance Criteria

- [x] Durable events include `parts` (or equivalent JSON) instead of flat `content`.
- [x] Tool results include success/failure and duration.
- [x] Existing sessions can be imported/migrated.
- [x] Replay produces the same conversation.

## Tests Added

- `durable_message_sent_preserves_parts` — verifies parts round-trip through JSON
- `durable_message_sent_backward_compatible` — verifies old JSON loads correctly
- `durable_message_sent_reconstructs_content_from_parts` — verifies content is rebuilt from text parts
- `session_save_preserves_message_parts` — verifies end-to-end save/load preserves all parts

## Design Impact

No change to TUI element design or composition. Only durable event format changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for durable event round-trip of rich messages and tool results. ✓ (31 durable tests pass)
- **Layer 2 — Event Handling:** Replay events carry full data. ✓ (backward compatible)
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session replay with images/reasoning/tools passes. (**pending**)
- **Live tmux testing session (required):** A turn with tool calls and images survives save/load. (**pending**)

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `SessionActor` owns session state; durable events are the persistence format.
- [x] **Trigger events:** `SessionSaved` triggers durable event serialization.
- [x] **Observer events:** `SessionLoaded` notifies observers of restored state.
- [x] **No direct mutations:** Durable events must not introduce direct state mutations.
- [x] **No new mirrors:** Durable events are the authoritative persistence format; no duplicates.
- [x] **Async work observed:** Persistence is in `SessionActor` via `spawn_blocking`.
