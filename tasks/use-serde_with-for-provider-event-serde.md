# Use serde_with for provider event serde

## Status

`todo`

## Context

`crates/runie-core/src/provider_event.rs:135-202` has ~70 lines of hand-written `Serialize`/`Deserialize` impls for `ModelError` that map enum variants to a JSON struct with `kind`/`message` fields.

## Goal

Replace with `serde_with` (`SerializeDisplay`/`DeserializeFromStr`) or derive-friendly serde attributes.

## Acceptance Criteria
- [x] Add `serde_with` dependency.
- [x] Replace manual impls.
- [x] Ensure durable JSON byte-compatibility.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for JSON round-trip.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider event tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (serde-only change, no TUI impact).
