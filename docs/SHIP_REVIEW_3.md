# Ship Review #3 — Code & Architecture Audit

**Date:** 2026-06-08  
**Mandate:** 80/20. Ship. Less code, more value.

---

## Good News: Dead Code is Gone

The ~3,265 lines of dead code identified in Ship Review #2 have been **deleted**:

| File/Dir | Status | Lines Removed |
|----------|--------|---------------|
| `event_bus.rs` | ✅ Deleted | 441 |
| `orchestrator.rs` | ✅ Deleted | 337 |
| `actors/` | ✅ Deleted | 1,521 |
| `session_jsonl.rs` | ✅ Deleted | 500 |
| `session_manager/` | ✅ Deleted | 466 |
| `render_generation` field | ✅ Removed | 1 |
| **Total** | | **3,266** |

**runie-core shrank from ~13,500 to 10,819 lines** (20% reduction).

`lib.rs` no longer exports dead modules. Build passes with only warnings.

---

## Bad News: 3 Tasks Are "Done" But Not Wired

### 1. `r1-config-keybindings` — MODULE EXISTS, NOT USED

**What's implemented:**
- `crates/runie-core/src/keybindings.rs` (233 lines)
- Parses `~/.runie/keybindings.json`
- Returns `HashMap<String, String>` (key combo → event name)
- 8 unit tests pass

**What's NOT implemented:**
- `main.rs` never calls `load_keybindings()`
- All key mapping is still hardcoded in `convert_event()` / `map_key_event()`
- Changing `keybindings.json` has **zero effect** on runtime behavior

**Fix:** Either wire it into `main.rs` (replace hardcoded map with loaded map) or mark the task as **not done**.

---

### 2. `mvp-config-hot-reload` — ZERO RUNTIME CODE

**What's implemented:**
- Nothing in the runtime.
- The only file watcher code was in `actors/config_agent.rs` (deleted with dead code).

**What's NOT implemented:**
- No `notify` crate dependency
- No file watcher thread
- No config re-parse on change
- No model/provider switch without restart

**Fix:** Mark as **not done** or delete the task. Config reload is low-value (restart is fine).

---

### 3. `r1-tui-chunk-events` — STREAMING WORKS, NOT PER-EVENT

**What's implemented:**
- `runie-provider` uses `on_chunk` callback for streaming
- `runie-agent/src/turn.rs` accumulates chunks into `AgentResponse`

**What's NOT implemented:**
- Chunks are NOT emitted as individual `CoreEvent`s to the event loop
- The event loop only sees the final `AgentResponse`, not per-chunk events
- The SPEC says "Each LLM chunk emitted as individual event" — this is not true

**Fix:** This may be intentional (accumulating chunks before state update is simpler and works). Mark SPEC/task as reflecting actual behavior, or remove the per-chunk requirement.

---

## Correctly Done Tasks

| Task | Evidence |
|------|----------|
| `r1-input-history-persistence` | `input_history.rs` loads on `AppState::default()`, saves on `Submit`. Properly wired in `update/input.rs`. |
| `r1-core-refactor` | `update.rs` split into `update/{mod,input,agent,slash,queue, line_nav}.rs`. Clippy clean. |
| `r1-tui-bash-prefix` | `update/input.rs:342` handles `content.starts_with('!')`. |
| `r1-tui-collapse-expand` | `main.rs` maps Ctrl+E → `ToggleExpand`. `update/mod.rs:137` handles it. |

---

## Architecture Health Check

### Current Runtime (main.rs)

```
┌─────────────────┐     CoreEvent      ┌─────────────────┐
│  input_reader   │ ──────────────────>│   event_loop    │
│  (crossterm)    │                    │   (owns state)  │
└─────────────────┘                    │                 │
                                       │  ┌───────────┐  │
┌─────────────────┐     CoreEvent      │  │ AppState  │  │
│   agent_loop    │ ──────────────────>│  └─────┬─────┘  │
│ (run_agent_turn)│                    │        │        │
└─────────────────┘                    │   snapshot()    │
                                       │        │        │
┌─────────────────┐     Snapshot       │  ┌─────┴─────┐  │
│  render_task    │ <──────────────────│  │ render_tx │  │
│   (ratatui)     │                    │  └───────────┘  │
└─────────────────┘                    └─────────────────┘
```

Three tokio tasks + one event loop. Simple, correct, no dead code.

### Module Count by Crate

| Crate | Lines | Files | Status |
|-------|-------|-------|--------|
| `runie-core` | 10,819 | 19 + tests | Clean |
| `runie-agent` | 1,772 | 8 | Clean |
| `runie-tui` | 2,671 | 9 + tests | Clean |
| `runie-term` | 2,028 | 1 + tests | Clean |
| `runie-provider` | 895 | 6 | Clean |
| **Total** | **~18,185** | | |

### Lint Status

- **File size:** All runie-core files ≤ 500 lines ✅
- **build.rs:** Still exists but doesn't fail (all files compliant)
- **Clippy:** Clean (zero errors, warnings only)

---

## Recommendations

### Immediate (Before Ship)

1. **Fix task statuses** — Mark these as **not done** or **partial**:
   - `r1-config-keybindings` — module exists, not wired
   - `mvp-config-hot-reload` — no runtime implementation
   - `r1-tui-chunk-events` — streaming works, not per-event

2. **Wire keybindings** (30 min) — Replace hardcoded `map_key_event()` with:
   ```rust
   let bindings = load_keybindings(&default_keybindings_path());
   // lookup in HashMap instead of match
   ```

3. **Delete or defer `mvp-config-hot-reload`** — No code exists. Low user value.

### Do Not Do

- Don't implement per-chunk events (current accumulation works fine)
- Don't add hot reload (restart is acceptable)
- Don't add more features — ship what works

---

## Task Inventory (Reality-Adjusted)

| Milestone | Actually Done | Partial | Not Done |
|-----------|---------------|---------|----------|
| **MVP** | 33 | 1 (`mvp-config-hot-reload`) | 0 |
| **R1** | 4 | 2 (`keybindings`, `chunk-events`) | 0 |
| **R2** | 0 | 0 | 1 (`queue-delivery-mode`) |
