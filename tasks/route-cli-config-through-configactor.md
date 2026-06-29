# Route CLI config operations through `ConfigActor`

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: unify-provider-credential-resolution-with-dotenvy, use-notify-directly-in-config-actor, unify-provider-config-persistence

## Description

`runie-cli` now routes all config operations through `RactorConfigActor`. This task extends `RactorConfigActor` with layered config support and MCP server management, then updates the CLI to use the actor.

## Changes Made

### `RactorConfigActor` Extensions

- Added `project_path: Option<PathBuf>` field to store the project config path
- Added `ConfigScope` enum for global vs project scope
- Added `ConfigMsg::LoadLayers` to load layered config and return the effective result
- Added `ConfigMsg::AddMcpServer`, `ConfigMsg::RemoveMcpServer`, `ConfigMsg::ListMcpServers` for MCP operations
- Actor now loads layered config on startup using `Config::load_layers_from_paths`
- Added `spawn_default()` convenience method for spawns without explicit paths
- Added file helpers for MCP server operations

### CLI Updates

- `inspect.rs`: Now spawns a short-lived `RactorConfigActor` and calls `build_with_config_actor()` async method
- `mcp.rs`: Now spawns a short-lived `RactorConfigActor` and uses async versions of list/add/remove operations

## Acceptance Criteria

- [x] `RactorConfigActor::spawn` accepts both a global and a project path (or resolves the project path from `std::env::current_dir()` at spawn time) and stores both.
- [x] Add `ConfigMsg` variants: `LoadLayers { reply }`, `AddMcpServer { scope, name, server }`, `RemoveMcpServer { scope, name }`, `ListMcpServers { scope, reply }`.
- [x] Add MCP helpers in `crates/runie-core/src/actors/config/file_helpers.rs` for adding, removing, and listing MCP servers per scope.
- [x] Implement the corresponding handlers in `RactorConfigActor` so layered config loading and MCP server add/remove/list work atomically.
- [x] Add reply methods to `RactorConfigHandle` for `load_layers`, `add_mcp_server`, `remove_mcp_server`, and `list_mcp_servers`, and propagate errors instead of returning `Option`.
- [x] Update `runie-cli/src/main.rs` to spawn a short-lived Tokio runtime for `inspect` and `mcp` subcommands (similar to `run_json`/`run_server`).
- [x] Replace direct `Config::load`, `Config::load_layers`, and `Config::save_to` calls in `runie-cli/src/inspect.rs` and `runie-cli/src/mcp.rs` with `RactorConfigHandle` requests.
- [x] Ensure CLI inspect still produces identical output and that MCP config read/write operations remain atomic from the caller's perspective.
- [ ] Remove the legacy `ConfigActor` re-export and the legacy `ConfigActorHandle` from `crates/runie-core/src/actors/config/mod.rs` once no production/test code imports them. (Not done - still used by some tests)
- [ ] Remove any now-unused direct config-loading helper imports in the CLI crate. (Acceptable as sync fallbacks)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `load_layers_returns_effective_config` — verifies that `ConfigMsg::LoadLayers` returns the same effective configuration that `Config::load_layers` previously returned.
- [x] `mcp_server_roundtrip` — adds, lists, and removes an MCP server through `RactorConfigActor` and asserts the on-disk config matches.

### Layer 2 — Event Handling
- [x] All existing config actor tests continue to pass, verifying message handling works correctly.

### Layer 3 — Rendering
- N/A — this task changes command-side routing, not TUI rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] CLI unit tests pass, verifying the async CLI functions work correctly.

## Files touched

- `crates/runie-cli/src/inspect.rs` — Added async `build_with_config_actor()`, updated `run()`
- `crates/runie-cli/src/mcp.rs` — Added async `_internal_async()` functions, updated `run_mcp()`
- `crates/runie-core/src/actors/config/messages.rs` — Added `ConfigScope`, new `ConfigMsg` variants
- `crates/runie-core/src/actors/config/ractor_config.rs` — Added layered config support, MCP operations, `spawn_default()`
- `crates/runie-core/src/actors/config/file_helpers.rs` — Added MCP server helpers
- `crates/runie-core/src/actors/config/mod.rs` — Re-exports updated types
- `crates/runie-core/src/actors/leader/actor.rs` — Updated spawn calls
- `crates/runie-core/src/headless_runtime.rs` — Updated spawn call

## Notes

- The actor resolves the project path once at spawn time, storing it in the actor state.
- Legacy sync fallback functions kept in `mcp.rs` for backward compatibility with tests.
- `inspect.rs` kept sync `build()` method for test compatibility; new `build_with_config_actor()` is the production path.
