# Dedupe scoped-model enable/disable handlers

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/dialog/toggle.rs` defines `handle_scoped_model_enable_all` and `handle_scoped_model_disable_all`. The bodies are identical except for `model.enabled = true` vs `model.enabled = false`.

## Acceptance Criteria

- [ ] A single `set_scoped_models_enabled(state: &mut AppState, enabled: bool)` helper replaces the two handlers.
- [ ] Both event handlers delegate to the helper.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `set_scoped_models_enabled_all_true` — all models become enabled.
- [ ] `set_scoped_models_enabled_all_false` — all models become disabled.

### Layer 2 — Event Handling
- [ ] `enable_all_event_still_works` — existing event still enables all models.
- [ ] `disable_all_event_still_works` — existing event still disables all models.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/update/dialog/toggle.rs`

## Notes

Trivial refactor; ensure `state.mark_dirty()` is still called exactly once.
