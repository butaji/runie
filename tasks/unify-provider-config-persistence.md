# Unify provider-config persistence helpers

**Status**: todo
**Milestone**: R2
**Category**: Provider / Configuration
**Priority**: P1

**Depends on**: route-cli-config-through-configactor, unify-provider-credential-resolution-with-dotenvy
**Blocks**: none

## Description

`crates/runie-core/src/provider/config.rs` and `crates/runie-core/src/actors/config/file_helpers.rs` both implement `RwLock`-guarded save/remove/list helpers that wrap `Config::load/save_to`. The two modules are nearly identical and drift easily. After `RactorConfigActor` owns config operations, provider config persistence should route through it or through a single shared helper.

## Acceptance Criteria

- [ ] Choose a single source of truth: either `actors/config/file_helpers.rs` becomes the only persistence helper, or all persistence goes through `RactorConfigActor` messages.
- [ ] Delete the duplicate helpers in `provider/config.rs` (or make it a thin re-export).
- [ ] Update all callers in `runie-provider`, `runie-cli`, and `runie-agent` to use the unified path.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `save_load_list_round_trip` — provider config can be saved, listed, and removed through the unified helper.
- [ ] `no_duplicate_helpers` — only one set of save/remove/list functions remains.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/provider/config.rs`
- `crates/runie-core/src/actors/config/file_helpers.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- Callers in `crates/runie-provider/`, `crates/runie-cli/`, `crates/runie-agent/`

## Notes

- This overlaps `route-cli-config-through-configactor.md` and `unify-provider-credential-resolution-with-dotenvy.md`; do it after those land to avoid colliding refactors.
- If `RactorConfigActor` becomes the single owner, provider config messages should be added to its protocol.
