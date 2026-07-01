# Add AgentCommand test builder

## Status

`todo`

## Context

`AgentCommand` is constructed manually ~25 times across tests (`runie-agent/src/tests/**/*.rs`, `subagent.rs`, `actor.rs`, `runie-testing/src/runner.rs`), repeating all 10 fields.

## Goal

Add a test-only builder in `runie-testing`, e.g. `AgentCommand::test("hello")` or `runie_testing::agent_cmd(content)`, with setters for provider/model/read_only/etc.

## Acceptance Criteria

- [ ] Add builder in `runie-testing`.
- [ ] Replace manual struct literals in tests.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test code changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
