# Consolidate `mark_dirty()` call sites

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`state.mark_dirty()` is called after nearly every handler arm — 19 times in `update/input/text.rs`, 16 in `update/input/nav.rs`, 13 in `update/login_flow.rs`, 8 each in `update/dialog/panel.rs` and `update/system.rs`, totaling ~100 call sites across `update/`. Every handler must remember to call it or the UI won't redraw. Replace the per-arm `mark_dirty()` calls with a single dirty flag set by handlers (or a `Dirty` return from the dispatcher) and checked once at the top-level `update` entry point. This removes ~100 boilerplate call sites and eliminates the "forgot to mark dirty" bug class.

## Acceptance Criteria

- [ ] Handlers set a dirty flag (e.g. `state.dirty = true` or return `bool`) instead of calling `mark_dirty()`.
- [ ] The top-level `AppState::update` (or `update::dispatch`) checks the flag once and calls the actual dirty side-effect (cache invalidation, redraw signal).
- [ ] `mark_dirty()` method either removed or reduced to a thin private setter.
- [ ] No handler in `update/` calls `mark_dirty()` directly (grep confirms zero call sites in `update/`).
- [ ] All existing tests pass — UI redraw behavior unchanged.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `update_sets_dirty_flag` — any state-mutating event sets `state.dirty = true`.
- [ ] `noop_event_does_not_dirty` — events that fall through `_ => {}` leave `dirty = false`.
- [ ] `dirty_flag_cleared_after_consume` — after the dispatcher processes the flag, `dirty` resets to false.

### Layer 2 — Event Handling
- [ ] `text_input_marks_dirty` — `InputEvent::Input('a')` sets dirty.
- [ ] `login_flow_start_marks_dirty` — `LoginFlowEvent::Start` sets dirty.
- [ ] `dialog_cancel_at_root_no_dirty_when_closable_false` — edge case: no dirty when panel is non-closable and no pop happens.

### Layer 3 — Rendering
- [ ] N/A — rendering behavior unchanged; existing render tests cover indirectly.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms no missed dirty paths (tests that check `mark_dirty` side-effects like cache invalidation still pass).

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — add `dirty` field, modify `mark_dirty`/`update`
- `crates/runie-core/src/update/mod.rs` — add dirty check in dispatcher
- `crates/runie-core/src/update/input/text.rs` — remove ~19 `mark_dirty()` calls
- `crates/runie-core/src/update/input/nav.rs` — remove ~16 calls
- `crates/runie-core/src/update/login_flow.rs` (or `login_flow/handlers.rs`) — remove ~13 calls
- `crates/runie-core/src/update/dialog/panel.rs` — remove ~8 calls
- `crates/runie-core/src/update/system.rs` — remove ~8 calls
- `crates/runie-core/src/update/session.rs` — remove ~7 calls
- `crates/runie-core/src/update/dialog/toggle.rs` — remove ~7 calls
- `crates/runie-core/src/update/dialog/open.rs` — remove ~6 calls
- `crates/runie-core/src/update/input/mod.rs` — remove ~6 calls
- `crates/runie-core/src/update/path_complete.rs` — remove ~5 calls

## Notes

Two implementation approaches: (A) `AppState::dirty: bool` field, handlers set `state.dirty = true`, dispatcher checks + clears; (B) handlers return `bool`/`Dirty` enum, dispatcher aggregates. Approach A is simpler and requires no signature changes — prefer it. Risk: some `mark_dirty()` calls trigger cache invalidation side-effects beyond just a flag — audit `mark_dirty()` implementation before removing. If `mark_dirty` does more than set a flag, move that logic into the dispatcher's dirty-consume path. This is the single biggest boilerplate reduction (~100 LOC removed).
