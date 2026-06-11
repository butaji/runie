# Ship Review #4 — Final Architecture Audit

**Date:** 2026-06-08
**Mandate:** 80/20. Ship. Less code, more value.

---

## Verdict: READY TO SHIP

All 43 real tasks are **done**. Zero todo tasks remain (excluding `TEMPLATE.md`).

---

## What Changed Since Ship Review #3

### 1. `mvp-config-hot-reload` — DONE

**Was:** `todo` (no runtime implementation)
**Now:** `done`

`crates/runie-core/src/config_reload.rs` (276 lines) implements polling-based
config watching:
- Polls `~/.runie/config.toml` every 2 seconds
- Parses TOML to extract provider/model
- Emits `Event::SwitchModel` to the event loop on change
- Wired in `main.rs`: `tokio::spawn(config_reload::spawn_config_watcher(...))`

### 2. `r1-config-keybindings` — CONFIRMED DONE

**Was:** `in-progress` (module existed but not wired)
**Now:** `done`

- `main.rs:42` calls `keybindings::load_keybindings(&None)`
- `main.rs:45` passes the HashMap to `input_reader()`
- `crates/runie-term/src/keymap.rs` (284 lines) uses the loaded bindings in
  `convert_event()` instead of hardcoded mapping

### 3. SPEC.md R1 Section — UPDATED

All R1 features now marked `[x]`:
- Configurable keybindings
- Streaming event per chunk
- Hot reload
- Input history persistence

---

## Code Health

### Dead Code Status

| File/Dir | Status |
|----------|--------|
| `event_bus.rs` | ✅ Deleted |
| `orchestrator.rs` | ✅ Deleted |
| `actors/` | ✅ Deleted |
| `session_jsonl.rs` | ✅ Deleted |
| `session_manager/` | ✅ Deleted |
| `render_generation` field | ✅ Removed |

### Runtime Architecture (Verified)

```
main.rs spawns 4 tasks:
  1. agent_loop(cmd_rx, agent_tx)       — run_agent_turn()
  2. input_reader(input_tx, keybindings) — crossterm → CoreEvent (with keybindings)
  3. render_task(terminal, render_rx)   — Snapshot → ratatui
  4. config_reload watcher              — polls config.toml, emits SwitchModel

+ event_loop(state, input_rx, agent_rx, cmd_tx, render_tx)
```

### Production Code Quality

- **unwrap/expect in production code:** None
  - `main.rs:198` expect is inside `#[tokio::test]`
  - `model.rs:357` and `update/mod.rs:15,52` are `unwrap_or()` (safe fallback)
- **panics:** None in production code
- **clippy:** Clean (zero errors)
- **file sizes:** All runie-core files ≤ 500 lines

### Codebase Size

| Crate | Lines |
|-------|-------|
| runie-core | 11,220 |
| runie-agent | 1,772 |
| runie-tui | 2,671 |
| runie-term | 2,115 |
| runie-provider | 895 |
| **Total** | **~18,673** |

---

## Feature Verification

| Feature | Task | Code Evidence | Status |
|---------|------|---------------|--------|
| Keybindings | `r1-config-keybindings` | `main.rs:42` loads, `keymap.rs` uses | ✅ Done |
| Hot reload | `mvp-config-hot-reload` | `config_reload.rs`, `main.rs:47` spawns | ✅ Done |
| Chunk events | `r1-tui-chunk-events` | `turn.rs:29` emits per chunk | ✅ Done |
| History persistence | `r1-input-history-persistence` | `input.rs:351` save, init loads | ✅ Done |
| Queue delivery | `r2-queue-delivery-mode` | `queue.rs:45,91` handles both modes | ✅ Done |
| Bash prefix | `r1-tui-bash-prefix` | `input.rs:342` handles `!` | ✅ Done |
| Collapse/expand | `r1-tui-collapse-expand` | `main.rs:179` Ctrl+E mapping | ✅ Done |

---

## Recommendations

### Ship Now

Nothing blocks release. All features are implemented and wired.

### Optional Post-Ship

1. **Replace polling with `notify` crate** — Current 2-second polling works but
   `notify` would be more efficient. Low priority.
2. **Remove `VisibleRegion`** — Still referenced by autoscroll tests. Not blocking.
3. **AppState composition** — 27 fields work fine. Refactor only if needed.

### Do Not Do

- No more features. Ship what exists.
- No more refactoring. Code works and is clean.
- No more architecture changes. Runtime is simple and correct.

---

## Task Inventory (Final)

| Milestone | Done | Todo |
|-----------|------|------|
| MVP | 34 | 0 |
| R1 | 7 | 0 |
| R2 | 1 | 0 |
| **Total** | **42** | **0** |

(Plus `cleanup-dead-code` and `TEMPLATE.md`)
