# Route CLI config operations through `ConfigActor`

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`runie-cli` currently performs direct configuration file I/O in at least two places: `runie-cli/src/inspect.rs:91` calls `Config::load_layers` directly, and `runie-cli/src/mcp.rs:227`/`:235` call `Config::load` and `Config::save_to` directly. Both `docs/Architecture.md` and `AGENTS.md` establish `ConfigActor` as the single owner of config files.

Current state as of Round 4:

- `RactorConfigActor` exists but only stores a single global `path` and calls `Config::load_async(Some(path))`. It does not yet understand **layered** config (`global + local`) or scoped MCP writes (`user` vs `project`).
- `file_helpers.rs` has no MCP add/remove/list helpers.
- `ConfigMsg` has no `LoadLayers`, `AddMcpServer`, `RemoveMcpServer`, or `ListMcpServers` variants.
- `RactorConfigHandle` exposes only `send`, `try_send`, `get_config`, and `get_configured_providers`, and silently drops reply errors.
- `runie-cli/src/main.rs:49,53` runs `inspect` and `mcp` synchronously; routing through an async actor requires a short-lived Tokio runtime.
- `runie-cli/src/mcp.rs:224–250` implements its own direct file I/O and path resolution.

This task extends `RactorConfigActor` with the messages that standalone CLI commands need, wraps `inspect`/`mcp` with `block_on`, and routes all inspect/MCP config operations through the actor.

## Acceptance Criteria

- [ ] `RactorConfigActor::spawn` accepts both a global and a project path (or resolves the project path from `std::env::current_dir()` at spawn time) and stores both.
- [ ] Add `ConfigMsg` variants: `LoadLayers { reply }`, `AddMcpServer { scope, name, server }`, `RemoveMcpServer { scope, name }`, `ListMcpServers { scope, reply }`.
- [ ] Add MCP helpers in `crates/runie-core/src/actors/config/file_helpers.rs` for adding, removing, and listing MCP servers per scope.
- [ ] Implement the corresponding handlers in `RactorConfigActor` so layered config loading and MCP server add/remove/list work atomically.
- [ ] Add reply methods to `RactorConfigHandle` for `load_layers`, `add_mcp_server`, `remove_mcp_server`, and `list_mcp_servers`, and propagate errors instead of returning `Option`.
- [ ] Update `runie-cli/src/main.rs` to spawn a short-lived Tokio runtime for `inspect` and `mcp` subcommands (similar to `run_json`/`run_server`).
- [ ] Replace direct `Config::load`, `Config::load_layers`, and `Config::save_to` calls in `runie-cli/src/inspect.rs` and `runie-cli/src/mcp.rs` with `RactorConfigHandle` requests.
- [ ] Ensure CLI inspect still produces identical output and that MCP config read/write operations remain atomic from the caller's perspective.
- [ ] Remove the legacy `ConfigActor` re-export and the legacy `ConfigActorHandle` from `crates/runie-core/src/actors/config/mod.rs` once no production/test code imports them.
- [ ] Remove any now-unused direct config-loading helper imports in the CLI crate.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_loads_layers` — verifies that `ConfigMsg::LoadLayers` returns the same effective configuration that `Config::load_layers` previously returned.
- [ ] `config_actor_mcp_server_roundtrip` — adds, lists, and removes an MCP server through `RactorConfigActor` and asserts the on-disk config matches.

### Layer 2 — Event Handling
- [ ] `cli_mcp_config_intent_reaches_config_actor` — constructs a CLI MCP config request, routes it through the handler, and asserts that the `RactorConfigActor` mailbox receives the corresponding message instead of the command touching the filesystem directly.

### Layer 3 — Rendering
- [ ] N/A — this task changes command-side routing, not TUI rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `cli_config_save_does_not_race_actor` — runs the inspect and MCP CLI commands against a temporary `RUNIE_HOME` with a `RactorConfigActor` replay fixture and asserts that file writes happen only via the actor and that no concurrent direct writes occur.

## Files touched

- `crates/runie-cli/src/inspect.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-cli/src/main.rs`
- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/config/file_helpers.rs`
- `crates/runie-core/src/actors/config/mod.rs`
- `crates/runie-core/src/actors/config/ractor_handle.rs` (if it exists)
- `docs/Architecture.md` (update any CLI/config diagrams or descriptions if they still show direct file access)

## Notes

- The preferred implementation is to reuse an existing `RactorConfigActor` handle when the CLI is already running inside the actor system; for standalone CLI commands, spawn a short-lived headless runtime with a single `RactorConfigActor`.
- `inspect` needs **layered** config (`global + local`). The actor must grow layered-load support; do not flatten the layers inside the actor.
- Decide whether the actor resolves the project path once at spawn time or whether each MCP message carries an absolute path.
- Rejected alternative: keeping direct file reads in `inspect` for performance. This violates the single-owner invariant and creates race conditions with a running TUI session.
- Out of scope: changing the `Config` serialization format or the on-disk layout. Only the call sites and actor message protocol move.
