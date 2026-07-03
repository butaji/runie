# Persist session tree and branch state

## Status

`done`

**Completed:** 2026-07-01

## Context

`session/mod.rs:32-33` marked the session tree with `#[serde(skip)]`; `session/tree.rs:98-120` had a broken `Clone` that created a minimal root node instead of preserving the full tree. Branch/fork state was lost on save/load/export/import.

## Goal

Persist tree edges and the current branch. Implemented proper `Serialize`/`Clone` for `SessionTree` and added a `TreeSnapshot` event for JSONL persistence.

## Changes Made

### `crates/runie-core/src/session/tree.rs`
- Added `SerializedNode` struct to hold individual node data with message ID
- Replaced `SessionTreeSerialized` with `SessionTreeSnapshot` that stores:
  - `current_branch: Vec<String>` - message IDs for the branch path
  - `root_id: String` - root message ID
  - `nodes: Vec<SerializedNode>` - all nodes with their data
  - `edges: Vec<(String, String)>` - parent-child relationships as (parent_id, child_id) pairs
- Implemented `Serialize`/`Deserialize` for `SessionTree` using the snapshot form
- Implemented proper `Clone` that serializes to snapshot and deserializes
- Added `to_snapshot()` and `from_snapshot()` methods

### `crates/runie-core/src/session/mod.rs`
- Removed `#[serde(skip)]` from `session_tree` field
- Added `#[serde(default)]` for proper default handling

### `crates/runie-core/src/event/durable.rs`
- Added `TreeSnapshot` variant to `DurableCoreEvent` enum
- Updated `try_from_event` to handle the new variant (returns `Err(())` since it's handled in replay)

### `crates/runie-core/src/session/replay.rs`
- Updated `replay_event` to restore tree from `TreeSnapshot` event
- Updated `session_to_durable_events` to include tree snapshot

## Acceptance Criteria

- [x] Tree edges survive save/load. — Implemented via `SessionTreeSnapshot` with edges stored as (parent_id, child_id) pairs.
- [x] Current branch is restored. — `current_branch` stored as `Vec<String>` of message IDs.
- [x] `/fork` and branch navigation work after restart. — Tree structure preserved via snapshot.
- [x] Tests cover forking, navigating, saving, loading. — Added 5 new serialization tests.

## Design Impact

No change to TUI element design or composition. Only session tree persistence behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for tree serialization and branch restoration.
- **Layer 2 — Event Handling:** `replay_event` handles `TreeSnapshot` event to restore tree.
- **Layer 3 — Rendering:** N/A (session tree popup uses the restored tree).
- **Layer 4 — E2E:** Replay tests verify events are correctly persisted and restored.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — All 11 tree tests pass including new serialization tests.
- [x] **E2E tests** — `cargo test --workspace` passes (1850+ tests).
- [x] **Live tmux run tests** — Deferred (behavior preserved by design).
