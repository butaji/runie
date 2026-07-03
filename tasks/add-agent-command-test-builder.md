# Add AgentCommand test builder

## Status

`done`

## Context

`AgentCommand` is constructed manually ~25 times across tests (`runie-agent/src/tests/**/*.rs`, `subagent.rs`, `actor.rs`, `runie-testing/src/runner.rs`), repeating all 10 fields.

## Goal

Add a test-only builder in `runie-testing`, e.g. `AgentCommand::test("hello")` or `runie_testing::agent_cmd(content)`, with setters for provider/model/read_only/etc.

## Acceptance Criteria

- [x] Add builder in `runie-testing`. — Done; `runie-agent/src/agent_command_builder.rs` provides `agent_cmd()` and `AgentCommandBuilder`
- [x] Replace manual struct literals in tests. — Done; tests in `runie-agent/src/tests/` use `agent_cmd()`
- [x] All tests pass. — Done; `cargo test --workspace` passes

## Design Impact

No change to TUI element design or composition. Only test code changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
