# Add MCP tool schema cache and connection manager

## Status

`todo`

## Context

Agent surveys show Codex caches MCP tool schemas by config fingerprint and uses a central connection manager with parallel startup/cancellation.

## Goal

Add a config-fingerprinted tool-schema cache and a central `McpConnectionManager` that owns server lifecycles.

## Acceptance Criteria
- [ ] Compute cache key from server config.
- [ ] Cache `tools/list` JSON on disk.
- [ ] Central manager with parallel startup and clean shutdown.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for cache key and manager lifecycle.
- **Layer 2 — Event Handling:** MCP facts emitted on startup.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** MCP server tests pass.
- **Live tmux testing session (required):** MCP tools available after TUI startup.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
