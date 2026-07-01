# Collapse `DurableCoreEvent` into the canonical `Event` enum

## Status

`todo`

## Description

`crates/runie-core/src/event/durable.rs` maintains a parallel `DurableCoreEvent` enum with ~300 lines of hand-written `TryFrom` conversions. A single canonical `Event` enum with `#[serde(skip)]` transient fields can replace both enums.

## Acceptance criteria

- `DurableCoreEvent` is deleted.
- Canonical `Event` variants are annotated for serde so transient variants/fields are skipped.
- JSONL persistence round-trips the same data as before.

## Tests

### Layer 1 — State/Logic
- Every transient `Event` variant serializes to `None`/`skip`.
- Every durable variant round-trips through JSON.

### Layer 2 — Event Handling
- Replaying a session from durable events produces the same `AppState` as before.
