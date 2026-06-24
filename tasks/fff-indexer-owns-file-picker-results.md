# FffIndexerActor owns file picker search results

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry
**Blocks**: none

## Description

`fff_file_results` and `fff_debounce` are mutated synchronously by dialog openers and file-picker input handlers. `FffIndexerActor` already exists and emits `Event::FffSearchResult`, but the file picker currently bypasses it. Route everything through the actor.

Current violators:
- `update/dialog/open.rs` — `open_at_file_picker` writes `fff_file_results` and `fff_debounce`.
- `update/dialog/file_picker.rs` — `rebuild_file_picker` writes the same fields.
- `update/dialog/fff.rs` — synchronous `query_fff_files` call.

## Acceptance criteria

- [ ] `FffIndexerActor` is spawned in the TUI runtime and its handle is stored in `ActorHandles` (see `actor-lifecycle-and-handle-registry`).
- [ ] `FffIndexerActor` is the only producer of `fff_file_results`/`fff_debounce` updates.
- [ ] `AppState.fff_file_results` and `fff_debounce` are private; reads go through immutable accessors.
- [ ] File picker sends `FffMsg::Search { query, counter }` to `FffIndexerActor`.
- [ ] `Event::FffSearchResult { counter, entries }` is consumed by a projection helper that updates the fields.
- [ ] Synchronous `query_fff_files` in dialog handlers is removed.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `fff_actor_search_result_updates_state` — `FffSearchResult` with matching counter updates `fff_file_results`.

### Layer 2 — Event Handling
- [ ] `file_picker_filter_sends_search_intent` — typing in file picker sends `FffMsg::Search`.

### Layer 3 — Rendering
- [ ] `file_picker_renders_search_results` — results appear in the picker panel.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/messages.rs` — add `Search` message if missing.
- `crates/runie-core/src/actors/fff_indexer/actor.rs` — ensure actor handles search and emits result.
- `crates/runie-core/src/model/state/app_state.rs` — private `fff_file_results`/`fff_debounce`.
- `crates/runie-core/src/update/dialog/open.rs` — send search intent.
- `crates/runie-core/src/update/dialog/file_picker.rs` — consume `FffSearchResult`.
- `crates/runie-core/src/update/dialog/fff.rs` — remove synchronous query or make it a helper for the actor.

## Notes

- Coordinate with `view-actor-owns-view-state`: the file picker is a dialog, but the result data belongs to `FffIndexerActor`.
