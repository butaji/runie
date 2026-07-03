# Rewrite AGENTS.md Layer-4 example and remove stale checksum row

## Status

`done`

## Context

`AGENTS.md:62-78` Layer-4 example references non-existent `AgentEvent::TurnComplete` and `run_agent_turn_with_skills`. `AGENTS.md:135-137` claims the build script enforces agent manifest checksums, but it does not.

## Goal

Rewrite the example to compile (use real testing APIs) and remove the false checksum row.

## Acceptance Criteria
- [ ] Replace example with `run_headless_cli`/`ReplayProvider`/`MockToolSkill`/`capture_events`.
- [ ] Remove stale checksum row.
- [ ] Verify example compiles when pasted into a test.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Example snippet compiles.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
