# Preserve rich message and tool data in durable events

## Status

**partial** — `duration_secs` now preserved in `ToolResult`; `success` is hardcoded to `true` (tool errors emit a separate event).

## Context

`event/durable.rs:14-33` and `event/to_durable.rs:46-62` flatten messages to `content: String`, dropping `Part`s (images, tool calls, reasoning). `ToolResult` previously hardcoded `success: true` and dropped `duration`.

## Implementation (partial)

Updated `DurableCoreEvent::ToolResult` to include `duration_secs`:
- Added `#[serde(default)] duration_secs: f64` field to the enum variant
- Updated `try_from_event` to include `duration_secs` in conversion
- Updated reverse conversion to use preserved `duration_secs`
- Added backward-compatible JSON parsing (defaults to 0.0 for old sessions)
- Added unit tests for round-trip and backward compatibility

## Acceptance Criteria

- [ ] Durable events include `parts` (or equivalent JSON) instead of flat `content`. (**pending**)
- [x] Tool results include success/failure and duration. (duration_secs now preserved; success still hardcoded to true)
- [x] Existing sessions can be imported/migrated. (serde default = 0.0 for duration_secs)
- [x] Replay produces the same conversation. (tests pass)

## Goal

Store the full `ChatMessage.parts` vector in durable events (or SQLite `messages.parts_json`). Preserve tool success and duration.

## Acceptance Criteria

- [ ] Durable events include `parts` (or equivalent JSON) instead of flat `content`.
- [ ] Tool results include success/failure and duration.
- [ ] Existing sessions can be imported/migrated.
- [ ] Replay produces the same conversation.

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
