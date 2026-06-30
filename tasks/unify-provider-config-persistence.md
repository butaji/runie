# Unify provider-config persistence helpers

**Status**: done
**Milestone**: R2
**Category**: Provider / Configuration
**Priority**: P1

**Depends on**: route-cli-config-through-configactor, unify-provider-credential-resolution-with-dotenvy
**Blocks**: none

## Description

`crates/runie-core/src/provider/config.rs` and `crates/runie-core/src/actors/config/file_helpers.rs` both had similar save/remove helpers. Unified them by making `provider/config.rs` delegate to `file_helpers.rs` for the actual logic while keeping the locking behavior for concurrent access safety.

## Changes

- Made `file_helpers` module public so it can be used by `provider/config.rs`.
- `save_provider_config` and `remove_provider_config` in `provider/config.rs` now delegate to `file_helpers` functions.
- Kept the `CONFIG_LOCK` in `provider/config.rs` for concurrent access safety in tests.

## Acceptance Criteria

- [x] `provider/config.rs` delegates to `file_helpers.rs` for save/remove operations.
- [x] All callers continue to work (tests pass).
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `get_provider_config_reads_saved_config` — round-trip works.
- [x] `concurrent_provider_saves_do_not_corrupt_config` — concurrent saves are safe.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files touched

- `crates/runie-core/src/actors/config/mod.rs` (made file_helpers public)
- `crates/runie-core/src/provider/config.rs` (delegate to file_helpers)
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
