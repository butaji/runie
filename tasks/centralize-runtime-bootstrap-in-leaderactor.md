# Centralize runtime bootstrap in `LeaderActor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: consolidate-actor-runtime-on-ractor
**Blocks**: none

## Description

Move runtime startup out of `runie-tui/src/main.rs` and `runie-cli/src/acp.rs` into a shared `Leader::start(...)` (or `Runtime::spawn`) entry point in `runie-core`. Remove the duplicate `RactorTurnActor` spawned by the TUI, replace the ad-hoc `RenderActor` emulation (manual `std::sync::mpsc` + `watch` + OS thread) with a single snapshot channel, and make `Leader` the single source of truth for actor lifetimes across both binaries.

## Acceptance Criteria

- [ ] `Leader` (or a new `Runtime`) exposes one shared `start(...)` async constructor that returns a fully wired `ActorHandles` and a shutdown signal.
- [ ] `runie-tui/src/main.rs` calls the shared bootstrap instead of manually building `ActorHandles` and spawning `AgentActor` plus a second turn actor.
- [ ] `runie-cli/src/acp.rs` replaces its local `spawn_runtime` with the shared bootstrap.
- [ ] Render path in the TUI uses a single snapshot channel from the runtime instead of the current manual `mpsc` + `watch` + OS thread emulation.
- [ ] Duplicate turn actor and any per-binary startup drift are removed.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_start_returns_complete_runtime` — verifies that `Leader::start` returns all required handles, channels, and a working shutdown token.

### Layer 2 — Event Handling
- [ ] `leader_start_spawns_all_actors` — drives `Leader::start` and confirms each production actor is alive and reachable by its typed `ActorRef`.

### Layer 3 — Rendering
- [ ] N/A — this task moves render wiring to a single channel; no widget output is changed.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tui_and_cli_share_bootstrap` — replays a provider fixture through both the TUI and CLI entry paths (using shared in-process harnesses) and asserts identical event ordering and no duplicate turn actors.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/leader/mod.rs`
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/acp.rs`
- `crates/runie-tui/src/render.rs` (if present)

## Notes

`Leader` currently exists but is only exercised by tests; this task promotes it to the canonical runtime root. The TUI render path today re-implements what should be a single `watch`/`broadcast` snapshot channel from the turn/session actors. Rejected alternative: leaving CLI and TUI bootstraps separate with duplicated helper functions — it has already drifted and is the source of the duplicate turn actor. Out of scope: changing the render widget itself or the CLI command parser.
