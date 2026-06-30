# Derive durable and headless events from canonical Event

## Status

`todo`

## Context

`event/durable.rs` (`DurableCoreEvent`), `event/headless.rs` (`HeadlessEvent`), and `provider_event.rs` (`ProviderEvent`) represent the same lifecycle with parallel enums and hand-written conversion tables. Data is lost in conversions.

## Goal

Make `Event` the single canonical enum. Derive `DurableCoreEvent` and `HeadlessEvent` as serde views (tagged/flattened subsets) or via a small proc-macro, instead of maintaining parallel enums.

## Acceptance Criteria

- [ ] Define canonical `Event` with all needed fields.
- [ ] Derive durable and headless serialization shapes.
- [ ] Ensure existing JSONL session files remain deserializable.
- [ ] Delete parallel enums and conversion tables.

## Design Impact

No change to TUI element design or composition. Only event serialization behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for serialization/deserialization of durable and headless views.
- **Layer 2 — Event Handling:** Events flow unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI output and session replay JSONL match expected shapes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
