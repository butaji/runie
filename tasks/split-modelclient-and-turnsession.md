# Split ModelClient and TurnSession

## Status

`todo`

## Context

No separation exists between a long-lived model client and a per-turn streaming session.

## Goal

Create session-scoped `ModelClient` holding auth/transport and per-turn `TurnSession` holding turn tokens/state.

## Acceptance Criteria
- [ ] Refactor `runie-core/src/actors/provider.rs` and `runie-agent/src/actor.rs`.
- [ ] Reuse HTTP/WebSocket connections across turns.
- [ ] Support transport fallback in `TurnSession`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for client/session lifecycle.
- **Layer 2 — Event Handling:** Actor messages use new types.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Multi-turn replay tests pass.
- **Live tmux validation:** Multi-turn chat with real provider is faster.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
