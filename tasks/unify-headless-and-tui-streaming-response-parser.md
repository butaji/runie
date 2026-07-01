# Unify headless and TUI streaming response parser

## Status

`todo`

## Context

`crates/runie-agent/src/headless/mod.rs:194-330` duplicates the streaming response state machine in `crates/runie-agent/src/stream_response.rs:35-172` (text accumulation, `ToolStream`, tool fallback, message building). Bug fixes must land in two places.

## Goal

Extract a provider-agnostic `stream_response_to<Publisher>` that both the TUI agent and headless runner call with different event publishers.

## Acceptance Criteria

- [ ] Delete `HeadlessStreamState` and `stream_headless_response`.
- [ ] Headless `execute_headless_tools` uses the shared parser.
- [ ] TUI and headless produce identical `ProviderEvent` sequences.
- [ ] All provider-replay tests pass.

## Design Impact

No change to TUI element design or composition. Only internal streaming parser changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for the shared parser with mock publisher.
- **Layer 2 — Event Handling:** Both paths emit the same events.
- **Layer 3 — Rendering:** `TestBackend` output unchanged.
- **Layer 4 — E2E:** Provider replay fixtures pass for both TUI and headless.
- **Live tmux testing session (required):** Run the same prompt in TUI and headless; outputs match.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
