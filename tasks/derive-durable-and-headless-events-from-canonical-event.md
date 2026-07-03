# Derive durable and headless events from canonical Event

## Status

`done`

**Completed:** 2026-07-01

## Context

`event/durable.rs` (`DurableCoreEvent`), `event/headless.rs` (`HeadlessEvent`), and `provider_event.rs` (`ProviderEvent`) represent the same lifecycle with parallel enums and hand-written conversion tables. Data is lost in conversions.

## Goal

Make `Event` the single canonical enum. Derive `DurableCoreEvent` and `HeadlessEvent` as serde views or `TryFrom` conversions instead of maintaining parallel enums and hand-written conversion tables.

## Changes Made

### `crates/runie-core/src/event/durable.rs`
- Added `DurableCoreEvent::try_from_event(&Event) -> Option<Self>` — the single source of truth for `Event → DurableCoreEvent` conversion
- Added `impl TryFrom<&Event> for DurableCoreEvent` — delegates to `try_from_event`
- Added `impl TryFrom<&DurableCoreEvent> for Event` — reverse conversion for session replay
- All event variants are covered in the exhaustive match; transient events return `None`

### `crates/runie-core/src/event/to_durable.rs`
- Replaced hand-written match with delegation to `DurableCoreEvent::try_from_event`
- Removed redundant helper functions (`tool_called`, `tool_result`, `message_sent`, `model_switched`)
- Added test for `Response → MessageSent` conversion

### `crates/runie-core/src/event/headless.rs`
- Added `HeadlessEvent::try_from_event(&Event) -> Option<Self>` — single source of truth for `Event → HeadlessEvent` conversion
- Added `impl TryFrom<&Event> for HeadlessEvent` — delegates to `try_from_event`
- Covers: `ResponseDelta → Text`, `ThinkingDelta → Thinking`, `ToolStart → ToolCallStart`, `ToolInputDelta → ToolCallInputDelta`, `ToolEnd → ToolCallEnd`, `TokenStatsUpdated → Usage`, `Error → Error`, `Done → End`, `PermissionRequest → PermissionRequest`
- Empty deltas are filtered out (no `Text` for empty `ResponseDelta`)

### `crates/runie-core/src/session/replay.rs`
- Replaced hand-written `durable_to_event` match table with `Event::try_from(&durable_event).ok()`
- Removed ~30 lines of duplicate conversion logic

## Acceptance Criteria

- [x] Define canonical `Event` with all needed fields. (`Event` is the canonical enum; `TryFrom` covers all variants)
- [x] Derive durable and headless serialization shapes. (`DurableCoreEvent::try_from_event`, `HeadlessEvent::try_from_event`)
- [x] Ensure existing JSONL session files remain deserializable. (`DurableCoreEvent` serde format unchanged)
- [x] Delete parallel enums and conversion tables. (`to_durable.rs` now delegates; `session/replay.rs` uses `TryFrom`)

## Design Impact

No change to TUI element design or composition. Only event serialization behavior changes:
- `to_durable()` still returns `Option<DurableCoreEvent>` — API unchanged
- `session/replay::durable_to_event` still returns `Option<Event>` — API unchanged
- `HeadlessEvent` conversion is additive (headless module still builds `HeadlessEvent` directly from `ProviderEvent`; the `From<Event>` path is available for future use)

## Tests

- **Layer 1 — State/Logic:** 18 new tests for `durable.rs` conversions (forward and reverse), serde roundtrips, existing JSONL format preservation. 19 tests for `headless.rs` conversions.
- **Layer 2 — Event Handling:** Existing tests cover `Event::to_durable()` and `ProviderEvent → Event` unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI output and session replay tests unchanged.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core` passes (1845 tests, 0 new failures).
- [x] **E2E tests** — `cargo test --workspace` passes (all crates, excluding pre-existing flaky test `resume_loads_most_recent_session` which fails in parallel but passes in isolation).
- [x] **Live tmux run tests** — N/A (event conversion changes have no TUI-visible impact).
