# Ship Review #3 — Changes Applied

**Date:** 2026-06-08

---

## Documents Created

| File | Purpose |
|------|---------|
| `docs/SHIP_REVIEW_3.md` | Architecture audit — dead code deleted, wiring gaps found |

## Documents Updated

| File | Changes |
|------|---------|
| `tasks/r1-config-keybindings.md` | Status: `done` → `in-progress`. Added "Not implemented" section. Module exists but main.rs never calls `load_keybindings()`. |
| `tasks/mvp-config-hot-reload.md` | Status: `done` → `todo`. Removed references to deleted dead code (actors/config_agent.rs, event_bus.rs). Acknowledged no runtime implementation exists. |
| `tasks/r1-tui-chunk-events.md` | Status: `done` (confirmed). Added implementation section showing `turn.rs` emits `AgentResponse` per chunk. |
| `docs/SPEC.md` | R1 section updated: keybindings marked `[~]` (partial), chunk events marked `[x]`, hot reload marked `[ ]`, history persistence marked `[x]`. |

## Task Status Corrections

| Task | Old Status | New Status | Reason |
|------|------------|------------|--------|
| `r1-config-keybindings` | done | **in-progress** | Module parses JSON but not wired into runtime |
| `mvp-config-hot-reload` | done | **todo** | Zero runtime implementation |
| `r1-tui-chunk-events` | done | **done** (confirmed) | `turn.rs` emits `AgentResponse` per provider chunk |
| `r2-queue-delivery-mode` | done | **done** (confirmed) | `DeliveryMode` enum, `steering_mode`/`follow_up_mode` fields, `queue.rs` handles both modes |

## New Task Created

| Task | Status | Purpose |
|------|--------|---------|
| `cleanup-dead-code` | done | Tracked deletion of ~3,266 lines of dead code from Ship Review #2 |

## Code Verified

- **Dead code deleted:** `event_bus.rs`, `orchestrator.rs`, `actors/`, `session_jsonl.rs`, `session_manager/`, `render_generation` field
- **New modules:** `keybindings.rs` (233 lines), `input_history.rs` (251 lines)
- **runie-core size:** 10,819 lines (was ~13,500)
- **Lint:** All runie-core files ≤ 500 lines. Build passes.
- **input_history:** Properly wired — loads on init, saves on submit
- **keybindings:** NOT wired — main.rs still hardcodes all key mappings
- **hot reload:** NOT implemented — no file watcher code in runtime

## Current Task Inventory

| Milestone | Done | In-Progress | Todo |
|-----------|------|-------------|------|
| **MVP** | 34 | 0 | 0 |
| **R1** | 6 | 1 | 1 |
| **R2** | 1 | 0 | 0 |

**Remaining to ship:**
1. Wire `keybindings.rs` into `main.rs` (~20 lines)
2. Implement or defer `mvp-config-hot-reload`
