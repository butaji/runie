# Route CLI config operations through `ConfigActor`

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: config-ssot-via-configactor, migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`runie-cli` currently performs direct configuration file I/O in at least two places: `runie-cli/src/inspect.rs` calls `Config::load_layers` directly, and `runie-cli/src/mcp.rs` calls `Config::load` and `Config::save_to` directly. Both `docs/Architecture.md` and `AGENTS.md` establish `ConfigActor` as the single owner of `~/.runie/config.toml`.

This task extends `ConfigActor` with the messages that standalone CLI commands need (`LoadLayers`, `AddMcpServer`, `RemoveMcpServer`, `ListMcpServers`), spawns a short-lived Tokio runtime for synchronous CLI subcommands, and routes all inspect/MCP config operations through the actor.

## Acceptance Criteria

- [ ] Add `ConfigMsg` variants: `LoadLayers { reply }`, `AddMcpServer { scope, name, server }`, `RemoveMcpServer { scope, name }`, `ListMcpServers { scope, reply }`.
- [ ] Implement the corresponding handlers in `ConfigActor` so layered config loading and MCP server add/remove/list work atomically.
- [ ] Update `runie-cli/src/main.rs` to spawn a short-lived Tokio runtime for `inspect` and `mcp` subcommands (similar to `run_json`/`run_server`).
- [ ] Replace direct `Config::load`, `Config::load_layers`, and `Config::save_to` calls in `runie-cli/src/inspect.rs` and `runie-cli/src/mcp.rs` with `ConfigActorHandle` requests.
- [ ] Ensure CLI inspect still produces identical output and that MCP config read/write operations remain atomic from the caller's perspective.
- [ ] Remove any now-unused direct config-loading helper imports in the CLI crate.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_loads_layers` — verifies that `ConfigMsg::LoadLayers` returns the same effective configuration that `Config::load_layers` previously returned.

### Layer 2 — Event Handling
- [ ] `cli_mcp_config_intent_reaches_config_actor` — constructs a CLI MCP config request, routes it through the handler, and asserts that the `ConfigActor` mailbox receives the corresponding message instead of the command touching the filesystem directly.

### Layer 3 — Rendering
- [ ] N/A — this task changes command-side routing, not TUI rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `cli_config_save_does_not_race_actor` — runs the inspect and MCP CLI commands against a temporary `RUNIE_HOME` with a `ConfigActor` replay fixture and asserts that file writes happen only via the actor and that no concurrent direct writes occur.

## Files touched

- `crates/runie-cli/src/inspect.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-cli/src/main.rs` (runtime setup if headless `ConfigActor` is introduced)
- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/config/actor.rs`
- `docs/Architecture.md` (update any CLI/config diagrams or descriptions if they still show direct file access)

## Notes

- The preferred implementation is to reuse an existing `ConfigActor` handle when the CLI is already running inside the actor system; for standalone CLI commands, spawn a short-lived headless runtime with a single `ConfigActor`.
- Rejected alternative: keeping direct file reads in `inspect` for performance. This violates the single-owner invariant and creates race conditions with a running TUI session.
- Out of scope: changing the `Config` serialization format, the on-disk layout, or the `ConfigActor`'s internal message protocol. Only the call sites move.
