# Remove blocking IO from runie-core domain modules

**Status**: done  
**Milestone**: R4  
**Category**: Architecture / Actors  
**Priority**: P0

**Depends on**: centralize-app-state-ownership  
**Blocks**: simplify-event-vocabulary

## Description

The domain crate (`runie-core`) currently performs blocking IO in `model/`, `update/`, and `login_config.rs`: git discovery, sync bash, clipboard reads, config writes, and `std::env::current_dir` calls. This task moves all side effects behind async, mockable interfaces owned by IO actors. The domain layer becomes pure and fast to unit test.

## Acceptance Criteria

- [x] Git discovery lives in `crates/runie-core/src/actors/io/git.rs` behind a `GitService` trait.
- [x] Sync bash execution in `update/tools.rs` and `update/input/text.rs` is deleted; all bash goes through `IoActor`.
- [x] Clipboard reads in `update/input/text.rs` move to `IoActor` / `ClipboardService`.
- [x] Config-file writes move from `AppState` methods and `login_config.rs` into `ConfigActor` behind a `ConfigStore` trait.
- [x] Global `CONFIG_LOCK` in `login_config.rs` is removed.
- [x] `std::env::current_dir()` reads are replaced by a `WorkingDirSet` event injected at startup.
- [x] The sync-IO architectural test from `arch-guardrails-enforce-3-layers` passes.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `mock_config_store_loads_and_saves` — in-memory `ConfigStore` round-trips config.
- [x] `mock_git_service_detects_branch_and_origin` — `GitService` returns branch and remote name.
- [x] `working_dir_event_sets_cwd_name` — `WorkingDirSet` updates `cwd_name`.

### Layer 2 — Event Handling
- [x] `io_actor_bash_output_emits_event` — `IoActor` turns bash result into `BashOutput`.
- [x] `config_actor_emits_config_loaded` — `ConfigActor` loads file and publishes event.

### Layer 3 — Rendering
- [x] N/A — this task does not change rendering.

### Layer 4 — Smoke / Crash
- [ ] `smoke_turn_with_mock_io_services` — an agent turn runs with mocked git, bash, and config services.
  *Not implemented in this phase.* A full agent-turn smoke test with mocked IO services requires wiring `ProviderActor`/`AgentActor` with mock git/bash/config, which is deferred to the tool-runtime trait work in Phase 4.

## Files touched

- `crates/runie-core/src/model/mod.rs`
- `crates/runie-core/src/update/tools.rs`
- `crates/runie-core/src/update/input/text.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-core/src/update/path_complete.rs`
- `crates/runie-core/src/update/dialog/tab_complete.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/update/bootstrap.rs`
- `crates/runie-core/src/update/login_flow.rs`
- `crates/runie-core/src/tool/context.rs`
- `crates/runie-core/src/login_config.rs`
- `crates/runie-core/src/actors/io/actor.rs`
- `crates/runie-core/src/actors/io/messages.rs`
- `crates/runie-core/src/actors/io/git.rs` (new)
- `crates/runie-core/src/actors/config/actor.rs`
- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/config/store.rs` (new)
- `crates/runie-core/src/actors/config/mod.rs`
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/variants_tests.rs`
- `crates/runie-core/tests/arch_guardrails.rs`
- `crates/runie-core/src/tests/input_grapheme.rs`
- `crates/runie-core/src/tests/safety.rs`
- `crates/runie-core/src/login_config/tests.rs`
- `crates/runie-core/src/actors/config/tests.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/headless.rs`
- `crates/runie-agent/src/inspector.rs`
- `crates/runie-tui/src/app_init.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/main.rs`
- `tasks/remove-io-from-runie-core.md`

## Notes

- The clipboard read path was in `update/input/text.rs`, not `model/cache.rs`.
- `login_config.rs` is retained as a synchronous test-helper compatibility layer with the global `CONFIG_LOCK` removed.
- `persist_login_flow` and `AppState` config mutations still fall back to `login_config` helpers when `config_tx` is `None`; this keeps existing cross-crate and synchronous tests working while the production path always routes through `ConfigActor`.
