# Route CLI config operations through `ConfigActor`

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: config-ssot-via-configactor
**Blocks**: none

## Description

`runie-cli` currently performs direct configuration file I/O in at least two places: `runie-cli/src/inspect.rs` calls `Config::load_layers` directly, and `runie-cli/src/mcp.rs` calls `Config::load` and `Config::save_to` directly. Both `docs/Architecture.md` and `AGENTS.md` establish `ConfigActor` as the single owner of `~/.runie/config.toml`. This task routes all CLI inspect and MCP config operations through `ConfigActor` so that no CLI command reads from or writes to the config file directly.

## Acceptance Criteria

- [ ] Identify every direct `Config::load`, `Config::load_layers`, and `Config::save_to` call in `runie-cli/src/inspect.rs` and `runie-cli/src/mcp.rs`.
- [ ] Replace direct file I/O with either an intent sent to an existing `ConfigActor` handle or a headless `ractor` runtime that spawns a `ConfigActor` and exposes its handle for the duration of the command.
- [ ] Ensure CLI inspect still produces identical output and that MCP config read/write operations remain atomic from the caller's perspective.
- [ ] Remove any now-unused direct config-loading helper imports in the CLI crate.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_owns_loaded_layers` — verifies that the `ConfigActor` state contains the same effective configuration layers that `inspect` previously obtained through `Config::load_layers`.

### Layer 2 — Event Handling
- [ ] `cli_mcp_config_intent_reaches_config_actor` — constructs a CLI MCP config event, routes it through the handler, and asserts that the `ConfigActor` mailbox receives the corresponding intent instead of the command touching the filesystem directly.

### Layer 3 — Rendering
- [ ] N/A — this task changes command-side routing, not TUI rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `cli_config_save_does_not_race_actor` — runs the inspect and MCP CLI commands against a temporary `RUNIE_HOME` with a `ConfigActor` replay fixture and asserts that file writes happen only via the actor and that no concurrent direct writes occur.

## Files touched

- `crates/runie-cli/src/inspect.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-cli/src/main.rs` (runtime setup if headless `ConfigActor` is introduced)
- `crates/runie-core/src/actors/config.rs` (or equivalent `ConfigActor` implementation)
- `docs/Architecture.md` (update any CLI/config diagrams or descriptions if they still show direct file access)

## Notes

- The preferred implementation is to reuse an existing `ConfigActor` handle when the CLI is already running inside the actor system; for standalone CLI commands, spawn a short-lived headless runtime with a single `ConfigActor`.
- Rejected alternative: keeping direct file reads in `inspect` for performance. This violates the single-owner invariant and creates race conditions with a running TUI session.
- Out of scope: changing the `Config` serialization format, the on-disk layout, or the `ConfigActor`'s internal message protocol. Only the call sites move.
