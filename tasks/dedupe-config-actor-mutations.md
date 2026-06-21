# Dedupe ConfigActor mutation pattern

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/actors/config/actor.rs` has four methods (`save_provider`, `remove_provider`, `set_default_model`, `set_provider_models`) that each clone strings/path, call `tokio::task::spawn_blocking`, and then `if result.is_ok() { self.load_and_emit(bus).await; }`. Error handling is currently swallowed.

## Acceptance Criteria

- [ ] A generic helper such as `mutate_config<F>(&mut self, bus, f)` replaces the repeated pattern.
- [ ] Errors are propagated or logged instead of silently dropped.
- [ ] All four mutations use the helper.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `mutate_config_helper_emits_event_on_success` — helper reloads config and emits event.
- [ ] `mutate_config_helper_reports_error` — helper does not swallow I/O errors.

### Layer 2 — Event Handling
- [ ] `save_provider_event_still_flows` — existing event still updates config.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs`

## Notes

The helper should take a closure that does the blocking file mutation and returns `anyhow::Result<()>`.
