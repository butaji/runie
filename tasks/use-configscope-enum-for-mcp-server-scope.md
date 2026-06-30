# Use ConfigScope enum for McpServer scope

## Status

`todo`

## Context

`crates/runie-core/src/config/mcp.rs` stores `McpServer.scope` as a `String` ("user"/"project"), while `crates/runie-core/src/actors/config/messages.rs` uses a typed `ConfigScope` enum. The CLI `main.rs` manually converts strings, leading to duplicate validation and invalid-value risk.

## Goal

Use the existing `ConfigScope` enum in `McpServer` and derive `clap::ValueEnum` for the CLI flag. Delete manual string conversions.

## Acceptance Criteria

- [ ] Change `McpServer.scope` from `String` to `ConfigScope`.
- [ ] Derive `clap::ValueEnum` for `ConfigScope`.
- [ ] Update CLI `main.rs` to use the typed flag.
- [ ] Update TOML serialization/deserialization to use the same string values.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `ConfigScope` serialization round-trip.
- **Layer 1:** Invalid scope string fails deserialization with a clear error.
- **Layer 2 — Event Handling:** `ConfigMsg::AddMcpServer` carries a typed scope.
- **Layer 3 — Rendering:** `TestBackend` snapshot of `/mcp list` shows scope labels correctly.
- **Layer 4 — E2E:** `runie mcp add --scope project ...` persists and reloads with project scope.
- **Live tmux validation:** Use the TUI `/mcp add` form; select project scope and confirm it is persisted.
