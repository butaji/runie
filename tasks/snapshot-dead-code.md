# Remove Snapshot Dead Code and Cache Per-Frame Vec Fields

**Status**: todo
**Milestone**: R2
**Category**: Core Architecture
**Priority**: P1
**Depends on**: resolve-merge-conflicts

## Description

`crates/runie-core/src/snapshot.rs` and `crates/runie-core/src/model.rs`
have two distinct performance / dead-code issues:

1. **`Snapshot::visible_scroll()` (lines 122-160) and
   `VisibleRegion` (lines 18-22) are dead code.** The render path in
   `runie-term/src/main.rs:130` calls
   `runie_tui::ui::draw_snapshot(f, &snap)`, which uses
   `Paragraph::scroll()` + `ScrollbarState`. The `visible_scroll()`
   method (returning a `VisibleRegion`) is never called from
   production code. The previous `REVIEW.md` flagged this as item
   #10; the dead code is still there.

2. **`AppState::snapshot()` (model.rs:625) rebuilds 5+ `Vec` fields
   per frame** without caching:
   - `palette_items` (filtered command list) — `AppState::palette_items()` (model.rs:374-405) does cache by filter string, but the cache is invalidated any time `open_dialog` changes
   - `model_selector_items` (line 410-432) — same caching pattern
   - `session_tree_items` (line 407-408) — no cache
   - `settings_items` — recomputed every frame via `crate::update::settings_dialog::build_setting_items(self)` (full rebuild of settings menu)
   - `pending_edits` — cloned
   - `scoped_models` — cloned
   - `image_attachments` — cloned
   - `auth_providers` — read from disk via `crate::auth::AuthStorage::load()` every frame

The render path runs ~30-60 Hz (200ms animation tick is the slowest
event, but snapshot is also rebuilt on every event). A fast provider
streaming 100+ chunks/second rebuilds the snapshot for each chunk.

## Acceptance Criteria

- [ ] `crates/runie-core/src/snapshot.rs` no longer contains:
  - The `VisibleRegion` struct
  - The `Snapshot::visible_scroll` method
  - The `Snapshot::visible` method (if it has the same dead signature)
- [ ] `git grep -n 'VisibleRegion\|visible_scroll' crates/` returns zero results (except possibly in CHANGELOG/REVIEW/archive dirs)
- [ ] `AppState::snapshot()` does not call `crate::auth::AuthStorage::load()` — auth provider list is cached in state and refreshed only on `LoginFlowSave` / `LoginFlowCancel` events
- [ ] `AppState::snapshot()` does not call `settings_dialog::build_setting_items` for the settings row data — settings items are cached in `AppState` (or `ViewState`) and invalidated only on `Event::SwitchTheme` / `Event::CycleThinkingLevel` / `Event::ToggleReadOnly` / `Event::SwitchModel`
- [ ] `session_tree_items` is cached in `AppState` and invalidated on `Event::ToggleSessionTree` / `Event::ForkSession` / `Event::CloneSession`
- [ ] The `Vec<(String, String, String)>` palette items are wrapped in `Arc<[…]>` so the snapshot doesn't clone the vec per frame
- [ ] The 3-line `Arc::clone` cost replaces the 30-line `Vec::clone` cost
- [ ] `cargo build --workspace` succeeds and the existing test suite still passes

## Tests

### Layer 1 — State/Logic
- [ ] `test_snapshot_does_not_call_auth_load` — instrument `AppState::snapshot` with a counter, call it 100 times, assert `auth::AuthStorage::load` was called ≤ 1 time (only on first call)
- [ ] `test_settings_items_cached` — same instrumentation pattern for `settings_dialog::build_setting_items`
- [ ] `test_session_tree_items_cached` — same for `session_tree::SessionTree::filtered_walk`
- [ ] `test_arc_sharing_works` — `let s1 = state.snapshot(); let s2 = state.snapshot(); assert!(Arc::ptr_eq(&s1.elements, &s2.elements));` (both snapshots share the same `elements` Arc)
- [ ] `test_visible_region_removed` — `Snapshot::visible_scroll` no longer exists; calling it is a compile error (the test file should not reference it)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib snapshot_optimization` passes (the existing `tests/snapshot_optimization.rs` test file covers caching semantics)
- [ ] `cargo test -p runie-core --lib tests::palette` passes (palette rendering with cached items)
- [ ] `cargo test -p runie-core --lib tests::settings_dialog` passes (settings rendering with cached items)

### Layer 3 — Rendering
- [ ] `cargo test -p runie-tui --lib` passes (rendering uses snapshots; this catches regressions)

### Layer 4 — Smoke
- [ ] A tmux script that streams a 1000-token response and measures frame time: average frame render < 16ms (60 fps)

## Notes

**Why `auth_providers` is the worst offender:** `AuthStorage::load()`
reads and parses `~/.runie/auth.toml` (or similar) from disk on
every snapshot. This is in the render path. The render actor runs in
a separate tokio task, but the snapshot it receives is built on the
event-loop side. The auth list changes only on login/logout — it
should be loaded once at startup and on those events.

**The 3 cheapest cacheable fields** (in priority order):

1. `auth_providers: Vec<String>` — disk read per frame, only changes on login events
2. `settings_items: Vec<SettingItem>` — full menu rebuild per frame, only changes on config events
3. `session_tree_items: Vec<(usize, String)>` — tree walk per frame, only changes on tree events

The `palette_items` and `model_selector_items` are *already* cached
by filter string — the cache is just invalidated too aggressively.
Either:
- Invalidate only on the events that actually change the filter (not on every dialog open/close)
- Or, since the dialog back-stack is the only place that changes the filter, invalidate only on `Event::PaletteFilter` / `Event::ModelSelectorFilter` / `Event::DialogBack`

**`Snapshot::visible_scroll` is a previous-review finding** that
was not addressed. This task is the fix. The function
unnecessarily walks `line_counts` to compute a `VisibleRegion`
struct that the render path doesn't consume. The render path uses
`Paragraph::scroll((scroll_offset as u16, 0))` (see
`crates/runie-tui/src/ui.rs` and the `scroll_offset` method on
`Snapshot`).

**The `Arc<[Element]>` for elements is already in place**
(`snapshot.rs:60`) — the work is to extend the same pattern to
`palette_items`, `model_selector_items`, `session_tree_items`,
`settings_items`. The change at the call site is:

```rust
// Before
pub palette_items: Vec<(String, String, String)>,

// After
pub palette_items: Arc<[(String, String, String)]>,
```

The `AppState::snapshot()` then `Arc::clone`s instead of `clone()`s.

**Out of scope:**
- Eliminating the per-message `Arc::clone` for `elements` (this is
  already cheap; the win is in the `Vec` fields)
- Coalescing multiple `AppState::update` calls into a single
  `snapshot` per render frame (architectural change)
- Replacing the `watch::channel` for snapshots with a `RwLock` (not
  a bottleneck)

**Verification:**
```bash
# No dead code
! git grep -nE 'VisibleRegion|visible_scroll' -- 'crates/'

# Snapshot tests still pass
cargo test -p runie-core --lib snapshot
cargo test -p runie-core --lib tests::palette
cargo test -p runie-core --lib tests::settings_dialog

# Build clean
cargo build --workspace
```
