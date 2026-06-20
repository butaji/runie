# Finish IO migration: remove remaining sync IO from runie-core domain

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: remove-login-config-test-shim
**Blocks**: delete-async-io-bridge

## Description

`remove-io-from-runie-core` is marked "done" but the `arch_guardrails.rs` test still carries a `legacy_sync_io_files()` allow-list, and real sync IO remains in the domain crate:

- `model/mod.rs:52,80,89` — `std::fs::read_to_string` for git detection (`detect_git_info`, `read_branch`, `read_origin_repo_name`). This is domain-layer code doing blocking file IO.
- `auth.rs:42,77,87` — `std::fs::read_to_string` / `create_dir_all` / `write` for auth token storage.
- `session_store.rs:87` — `fs::read_to_string` for legacy JSONL migration.
- `model/mod.rs:109` — `std::env::current_dir()` in `current_dir_name()`.

The 3-layer rule (IO | Domain | UI) is documented but enforced only by a source-scan test with an allow-list that silently permits the violations. The `GitService` trait exists in `actors/io/git.rs` but `model/mod.rs` bypasses it. This task finishes the migration: move every remaining sync IO call behind an existing actor trait (`GitService`, `ConfigStore`, `SessionStore`) and delete the corresponding allow-list entries so the guardrail actually enforces the rule.

## Acceptance Criteria

- [ ] `model/mod.rs` git detection (`detect_git_info`, `read_branch`, `read_origin_repo_name`, `read_git_info`, `read_worktree_git_info`) moved behind `GitService` trait; domain `model/mod.rs` contains no `std::fs::` calls.
- [ ] `auth.rs` file IO moved behind a `ConfigStore`-or-dedicated `AuthStore` trait; no `std::fs::` in `auth.rs`.
- [ ] `session_store.rs` legacy JSONL migration moved into `spawn_blocking` inside the (unified) `SessionActor`; no `std::fs::` in domain `session_store.rs` beyond the redb path constructor.
- [ ] `model/mod.rs::current_dir_name()` removed; `cwd_name` is set via `WorkingDirSet` event at startup (already wired in `update/bootstrap.rs`).
- [ ] `legacy_sync_io_files()` allow-list in `arch_guardrails.rs` emptied (or reduced to only the `actors/io/` adapter modules that intentionally own sync IO).
- [ ] `arch_test_no_sync_io_in_core` fails if new sync IO appears anywhere in `crates/runie-core/src` outside `actors/io/`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `mock_git_service_detects_branch_and_origin` — `GitService` mock returns branch + repo name; `model/mod.rs` no longer calls `std::fs` directly.
- [ ] `working_dir_event_sets_cwd_name` — `WorkingDirSet` event updates `cwd_name` (already exists, keep green).
- [ ] `auth_store_round_trips_token` — `AuthStore` mock saves + loads a token without disk IO.

### Layer 2 — Event Handling
- [ ] `git_info_loaded_event_sets_git_info` — `GitInfoLoaded` event from `IoActor` updates `AppState.git_info`.
- [ ] `auth_token_loaded_event_sets_token` — `AuthTokenLoaded` event updates auth state.

### Layer 3 — Rendering
- N/A — IO migration, no rendering change.

### Layer 4 — Smoke / Crash
- [ ] `smoke_turn_with_mock_io_services` — an agent turn runs with mocked git, bash, and config services (the Layer 4 test deferred from `remove-io-from-runie-core`).
- [ ] `arch_test_no_sync_io_in_core` — guardrail passes with empty (or adapter-only) allow-list.

## Files touched

- `crates/runie-core/src/model/mod.rs` — remove git IO; keep pure `GitInfo` type.
- `crates/runie-core/src/auth.rs` — move IO behind `AuthStore` trait.
- `crates/runie-core/src/session_store.rs` — move JSONL migration into actor.
- `crates/runie-core/src/actors/io/git.rs` — absorb `detect_git_info` logic.
- `crates/runie-core/src/actors/config/store.rs` — (possibly) absorb auth storage.
- `crates/runie-core/tests/arch_guardrails.rs` — empty `legacy_sync_io_files()`.
- `crates/runie-tui/src/app_init.rs` — wire `GitInfoLoaded` from `IoActor` startup.

## Notes

This is the structural foundation: until sync IO is gone from the domain crate, the "3-layer" rule is aspirational and the `async_io.rs` bridge helpers (tracked by `delete-async-io-bridge`) cannot be removed. Once this lands, `delete-async-io-bridge` becomes unblocked. The larger question of splitting `runie-core` into `runie-domain` + `runie-io` crates is deferred — finishing the IO migration is the prerequisite and delivers most of the purity benefit without the crate-split churn. Depends on `remove-login-config-test-shim` because `login_config.rs` is itself a sync-IO allow-list entry.
