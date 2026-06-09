# Optimize: Reduce Snapshot Cloning Overhead

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture

## Description

`Snapshot::new_from_state()` (in `AppState::snapshot()`) clones entire vectors and rebuilds data structures every frame:

- `elements_cache.clone()` — full Vec<Element> clone
- `line_counts.clone()` — full Vec<usize> clone  
- `palette_items()` — rebuilds entire palette filter on every frame
- `model_selector_items()` — rebuilds model list on every frame

With long conversations (100+ messages), this causes unnecessary allocation/GC pressure. Use `Arc` for immutable shared data or implement dirty-tracking to only clone changed parts.

## Acceptance Criteria

- [ ] Option A: Use `Arc<[Element]>` for snapshot's elements (no Arc<Vec> to avoid atomic refcount on every write)
- [ ] Option B: Implement per-field dirty flags, only clone dirty fields
- [ ] `palette_items` and `model_selector_items` cached with dirty invalidation
- [ ] Benchmark before/after: `cargo bench` for snapshot creation time
- [ ] No visible regression in UI responsiveness

## Tests

### Layer 1 — State/Logic
- [ ] `test_snapshot_contains_expected_fields` — verify all fields present
- [ ] `test_snapshot_is_send_sync` — required for channel transfer

### Layer 2 — Event Handling
- [ ] `test_event_triggers_snapshot_update` — verify snapshot refreshed on events

### Layer 3 — Rendering
- [ ] `test_render_receives_valid_snapshot` — verify render sees consistent data

### Layer 4 — Smoke
- [ ] `smoke_long_conversation.sh` — 50+ messages, verify no slowdown

## Notes

The cleanest solution is Option A: wrap `elements` in `Arc` since it only changes when `messages_changed()` is called, not on every keystroke.

For Option B, add a `snapshot_dirty: bool` flag and set it true on `messages_changed()`. Only then rebuild `palette_items` and `model_selector_items`.

**Out of scope**: Changing Snapshot structure sent to render actor (must remain backward compatible for render actor interface)
