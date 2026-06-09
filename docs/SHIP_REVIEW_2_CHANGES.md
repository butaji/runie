# Ship Review #2 — Changes Applied

**Date:** 2026-06-08

---

## Documents Created

| File | Purpose |
|------|---------|
| `docs/SHIP_REVIEW_2.md` | Full architecture & code audit |

## Documents Updated

| File | Changes |
|------|---------|
| `docs/SPEC.md` | Replaced fantasy actor architecture with actual runtime diagram; updated R1 section to remove cut tasks; marked `Ctrl+Shift+E` and `!command` as done; updated file structure; fixed Notes section |
| `tasks/r1-config-keybindings.md` | Removed ConfigAgent dependency; simplified to file read + HashMap |
| `tasks/mvp-config-hot-reload.md` | Removed "ConfigChanged events emitted to bus" (no bus); simplified to file watcher + re-parse |

## SPEC.md Changes Detail

### Architecture Section
- **Before:** Actor hierarchy diagram (EventBus → Orchestrator → 10 actors)
- **After:** Actual runtime diagram (3 tokio tasks + event loop with mpsc channels)
- **Added:** Historical note explaining the unused actor system

### MVP Section
- **Before:** "Actor-based event-driven architecture" and "Orchestrator spawning all actors"
- **After:** "Event-driven architecture (tokio async, mpsc channels)" and "Async runtime with non-blocking render"

### R1 Section
- **Removed:** Module split, render cleanup, all 4 actor tasks
- **Marked done:** Ctrl+Shift+E, !command
- **Added:** Prioritized remaining tasks (keybindings, chunk events, hot reload, history persistence)
- **Moved to Deferred:** AppState composition, VisibleRegion cleanup

### File Structure
- **Removed:** Fantasy paths (`ui/input_agent.rs`, `bus.rs`, `orchestrator.rs`, `queue_agent.rs`)
- **Added:** Actual paths (`update/`, `turn.rs`, `grep_find.rs`, `syntax.rs`, `diff.rs`)

## Task Updates

| Task | Change |
|------|--------|
| `r1-config-keybindings` | Removed ConfigAgent reference. Simplified to: read JSON → HashMap → dispatch |
| `mvp-config-hot-reload` | Removed "ConfigChanged events" and "Actors apply changes". Simplified to: file watcher → re-parse TOML |

## Findings Documented (Not Yet Implemented in Code)

These require code changes and are tracked in `docs/SHIP_REVIEW_2.md`:

1. **Delete dead modules** (~3,265 lines):
   - `event_bus.rs`, `orchestrator.rs`, `actors/`
   - `session_jsonl.rs`, `session_manager/`
   - Remove from `lib.rs`

2. **Delete dead field:**
   - `render_generation` in AppState

3. **Fix build.rs:**
   - Move lint from build gate to CI/`cargo xtask`
   - Currently only checks runie-core, not runie-tui where syntax.rs violates limit

4. ~~**Trim syntax.rs**~~ — Already split into `syntax/` directory. No lint violations remain.

## Task Inventory (Post-Review)

| Milestone | Done | Todo |
|-----------|------|------|
| MVP | 35 | 1 |
| R1 | 3 | 5 |
| R2 | 0 | 1 |

**R1 todo (prioritized):**
1. `r1-config-keybindings` — Highest user value
2. `r1-tui-chunk-events` — Medium value, low effort
3. `r1-tui-collapse-expand` — Medium value, low effort
4. `mvp-config-hot-reload` — Low value
5. `r1-input-history-persistence` — Low value
