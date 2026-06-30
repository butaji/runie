# Unify provider retry on backon including streams

## Status

`todo`

## Context

API-key validation uses workspace `backon`, but streaming uses `reqwest_eventsource::retry::ExponentialBackoff` (`provider/src/openai/stream.rs:87-96`). Two different retry policies and implementations.

## Goal

Unify retry on `backon`: retry the stream establishment request with `backon`, then surface stream errors immediately once SSE bytes start flowing.

## Acceptance Criteria

- [ ] Remove `reqwest_eventsource` backoff configuration.
- [ ] Use `backon` for stream-establishment retries.
- [ ] Keep "no byte-level retry once SSE starts" rule.
- [ ] All provider retry tests pass.

## Design Impact

No change to TUI element design or composition. Only provider retry behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for retry predicates and backoff.
- **Layer 2 — Event Handling:** Retry events surface correctly.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay fixture with transient failure retries.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
