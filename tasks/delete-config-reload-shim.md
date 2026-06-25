# Delete Config Reload Shim

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: config-ssot-via-configactor
**Blocks**: consolidate-config-modules-into-dir

## Description

Remove the config reload shim from `AppState`. With `ConfigActor` as the SSOT, `AppState.config_cache` is updated only via `ConfigLoaded` facts, not through a separate reload mechanism.

## Acceptance Criteria

- [ ] `config_reload` field/method removed from `AppState`
- [ ] Config reload goes through `ConfigActor`
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `config_reload_removed`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-core/src/config.rs`

## Notes

- Simple deletion task after `config-ssot-via-configactor` is complete
