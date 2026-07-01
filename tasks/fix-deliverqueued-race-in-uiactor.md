# Fix DeliverQueued race in UiActor

## Status

`todo`

## Context

`crates/runie-tui/src/ui_actor.rs:633-678` sends `DeliverQueued`, then creates a fresh bus subscription and waits up to 100 ms for follow-up events, missing events published before subscription.

## Goal

Make queue delivery atomic or request/response so the actor waits for completion instead of polling.

## Acceptance Criteria
- [ ] Eliminate late-subscription race.
- [ ] Remove 100 ms polling.
- [ ] Queued turns still start correctly.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for queued turn delivery.
- **Layer 2 — Event Handling:** `DeliverQueued` returns a completion fact.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Multi-turn replay tests pass.
- **Live tmux testing session (required):** Queue multiple messages; they all deliver.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
