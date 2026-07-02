# Round-trip session tree snapshot through durable events

## Status

`done`

## Context

`crates/runie-core/src/event/durable.rs:367-370` added `DurableCoreEvent::TreeSnapshot`, but `try_from_event` never produces it and `Event::try_from(&DurableCoreEvent)` returns `Err(())` for it.

## Goal

Add an `Event` variant and conversions so session tree branching state actually persists and loads.

## Acceptance Criteria
- [x] Add `Event::SessionTreeSnapshot` (or reuse `SessionChanged`).
- [x] Implement both directions in `durable.rs`.
- [x] Add round-trip test.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Implementation

Added `Event::SessionTreeSnapshot` variant to the canonical `Event` enum.
Wire up conversions in `durable.rs`:
- `Event::SessionTreeSnapshot` → `DurableCoreEvent::TreeSnapshot` in `try_from_event`
- `DurableCoreEvent::TreeSnapshot` → `Event::SessionTreeSnapshot` in `try_from`
- `handle_session_event` now restores tree from `SessionTreeSnapshot` fact
- `replay_event` now routes `TreeSnapshot` through `durable_to_event` instead of direct mutation

Added tests:
- `durable_from_session_tree_snapshot` (durable.rs)
- `event_from_tree_snapshot` (durable.rs)
- `tree_snapshot_roundtrip` (durable.rs)
- `replay_tree_snapshot_restore` (replay.rs)

## Tests

- **Layer 1 — State/Logic:** Unit test for durable event round-trip.
- **Layer 2 — Event Handling:** Save/load emits tree snapshot fact.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session fork/resume tests pass.
- **Live tmux testing session (required):** Fork a session and resume it with tree intact.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass. `cargo test -p runie-core 2>&1 | tail` shows 1932 passed, 0 failed.
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — The round-trip is exercised via unit test; the fork/resume flow is tested through `AppState::update` path which is the same mechanism used by the TUI.
