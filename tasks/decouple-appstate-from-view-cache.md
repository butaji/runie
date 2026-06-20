# Decouple AppState from view-AST cache

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: fold-state-into-model-state
**Blocks**: rename-core-ui-to-view

## Description

`crates/runie-core/src/ui/` (`elements.rs`, `transform.rs`, `posts.rs`, `dsl_test.rs`) is the view-model: an `Element`/`Feed`/`Post` tree projected from `AppState` and cached via `LazyCache` **inside** `AppState` itself. `ui/mod.rs` admits this is a layering violation kept "for pragmatic reasons." The real problem is not the directory name (that is `rename-core-ui-to-view`); it is that the domain model (`AppState`) holds a cached view AST, so domain depends on view-IR types. This couples the IO | Domain | UI layers at the type level: any change to `Element`/`Post` forces `AppState` to recompile, and any domain test transitively links the view projection.

Fix the coupling first. Either (a) move the `LazyCache<Element>` out of `AppState` into `UiActor` (domain holds raw posts; `UiActor` owns the projection cache), or (b) if the projection must stay in core, reframe `ui/` as a domain-neutral `ir/` module with no `&mut AppState` and no `AppState` field of type `LazyCache`. Option (a) is preferred — it makes the layer split real.

## Acceptance Criteria

- [ ] `AppState` no longer has a field of type `LazyCache` / `Feed` / `Element` / `Post` (whichever is cached today).
- [ ] The view projection (`transform::to_elements` / `LazyCache`) is owned by `UiActor` (or `runie-tui`), constructed from `AppState::snapshot()` output, not from `&mut AppState`.
- [ ] `crates/runie-core` no longer references `Element`/`Feed`/`Post` from `AppState`'s public API. The types may still live in core as a neutral `ir/` module if option (b) is chosen, but `AppState` does not own them.
- [ ] `arch_guardrails.rs` gains a test asserting `AppState` struct fields contain no `LazyCache`/`Element`/`Feed`/`Post` types.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `app_state_has_no_view_cache_field` — parse `AppState` struct definition; assert no field type contains `LazyCache`, `Element`, `Feed`, or `Post`.
- [ ] `snapshot_does_not_build_elements` — `AppState::snapshot(&self)` returns a `Snapshot` with raw post data, not an `Element` tree.

### Layer 2 — Event Handling
- [ ] `event_updates_do_not_touch_view_cache` — applying an event to `AppState` does not invalidate or rebuild any view cache (asserted by absence of `LazyCache::invalidate` calls in `update/`).

### Layer 3 — Rendering
- [ ] `draw_snapshot_builds_elements_outside_appstate` — `runie-tui` constructs the `Element` tree from `Snapshot` (or via `UiActor`-owned `LazyCache`), not from `AppState`.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tui_renders_after_decouple` — TUI draws a frame from a snapshot without `AppState` holding the element cache.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — remove `LazyCache`/view-AST field
- `crates/runie-core/src/model/cache.rs` — remove feed-cache logic if owned here
- `crates/runie-core/src/ui/transform.rs` — projection moves to `UiActor` or becomes `ir/` with no `&mut AppState`
- `crates/runie-core/src/snapshot.rs` — `Snapshot` carries raw post data the view needs
- `crates/runie-tui/src/ui_actor.rs` — owns the `LazyCache<Element>` and builds it from snapshots
- `crates/runie-core/tests/arch_guardrails.rs` — new assertion

## Notes

Supersedes the structural intent of `rename-core-ui-to-view` (rename is cosmetic; this fixes the coupling). If option (a) lands, `rename-core-ui-to-view` becomes a follow-up rename of the now-tui-owned projection. If option (b) lands, rename `ui/` → `ir/` instead. The `ui/mod.rs` comment ("UI AST remains in runie-core because AppState caches the feed") is the exact debt this task removes. Related: `inline-or-document-core-ui-shim` (the `runie-tui/src/core_ui/` re-export) becomes moot once the projection lives in tui.
