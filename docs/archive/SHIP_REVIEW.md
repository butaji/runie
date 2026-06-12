# Ship Review: 80/20 Cut List

**Date:** 2026-06-08  
**Principle:** Less code, same value. Delete dead code, defer marginal features.

---

## The Big Finding: 24% of runie-core is Dead or Duplicate

The "actor architecture" built as MVP core infrastructure is **never used by the runtime**. The actual app in `main.rs` uses simple `tokio::spawn()` + `mpsc::channel()` patterns. The EventBus, Orchestrator, and 6 actor implementations are completely dead.

### Dead Code Inventory

| Module | Lines | Status | Notes |
|--------|-------|--------|-------|
| `event_bus.rs` | 441 | **DEAD** | Runtime uses `mpsc::channel`, not typed actor channels |
| `orchestrator.rs` | 337 | **DEAD** | Never instantiated by `main.rs` |
| `actors/` | 1,521 | **DEAD** | 6 actors + tool wrappers; never spawned |
| `session_jsonl.rs` | 500 | **UNUSED** | Only used by `session_manager/` actor wrapper |
| `session_manager/` | 466 | **UNUSED** | Actor-based session system; runtime uses `session.rs` instead |
| **Total** | **3,265** | | **24.2% of runie-core** |

### The Actual Runtime (What Works)

```rust
// main.rs — the real architecture
tokio::spawn(agent_loop(cmd_rx, agent_tx));
tokio::spawn(input_reader(input_tx));
tokio::spawn(render_task(terminal, render_rx));
// Simple event loop with direct AppState mutation
```

Sessions work through `session.rs` (230 lines, simple JSON) — slash commands call `crate::session::save/load/list/delete` directly.

The `session_jsonl.rs` + `session_manager/` event-log system is a parallel implementation that's never reached from the runtime.

### What to Delete

1. **`event_bus.rs`** — Delete entirely. Remove `pub mod event_bus` from `lib.rs`.
2. **`orchestrator.rs`** — Delete entirely. Remove `pub mod orchestrator` from `lib.rs`.
3. **`actors/`** — Delete entire directory. Remove `pub mod actors` from `lib.rs`.
4. **`session_jsonl.rs`** — Delete. Runtime uses `session.rs`.
5. **`session_manager/`** — Delete. Runtime uses `session.rs`.

**Net result:** −3,265 lines, +0 broken features.

**Tests lost:** 32 (all testing dead code — no value lost).

---

## Tasks to Delete or Restructure

### Delete These Tasks (They Track Dead Code)

| Task | Why Delete |
|------|------------|
| `mvp-core-bus` | Built `event_bus.rs` (dead) |
| `mvp-core-orchestrator` | Built `orchestrator.rs` (dead) |
| `mvp-core-event-unification` | Unified events into event_bus types (dead) |
| `mvp-session-jsonl` | Built `session_jsonl.rs` (unused by runtime) |
| `mvp-core-session-persistence` | Partially built dead session_manager system |
| `r1-actor-infrastructure` | Plans to implement actors (dead architecture) |

### Keep But Rename/Rescope

| Task | Action |
|------|--------|
| `mvp-config-hot-reload` | Keep as R1. Simple file watcher + reload, no actors needed. |
| `r1-core-refactor` | Scope to **done**. AppState composition is nice-to-have, not blocking. |
| `r1-agent-module-split` | **Delete**. Agents are not actors; module split has no user value. |
| `r1-tui-render-cleanup` | **Delete**. Test organization is not shipping work. |

### The Real R1 Should Be

| Priority | Task | User Value |
|----------|------|------------|
| P0 | `r1-config-keybindings` | High — users want custom keys |
| P1 | `r1-tui-bash-prefix` | Medium — `!command` is useful |
| P1 | `r1-tui-chunk-events` | Medium — better streaming UX |
| P2 | `r1-tui-collapse-expand` | Medium — Ctrl+Shift+E toggle |
| P2 | `r1-input-history-persistence` | Low — nice-to-have |
| P3 | `mvp-config-hot-reload` | Low — restart is fine |
| P3 | `mvp-input-multiline-cursor` | Low — backspace works in most cases |

---

## SPEC.md Cleanup

### Remove These Sections

1. **Actor Architecture diagram** in SPEC.md — Replace with actual runtime diagram (tokio spawn + channels)
2. **ADR-0001 through ADR-0012** — Most describe the actor system. Keep ADR-0013 (keybindings) and ADR-0011 (non-interactive modes).

### Keep These ADRs (Still Valid)

| ADR | Status |
|-----|--------|
| `0011-non-interactive-modes-separate-binaries` | Valid future direction |
| `0013-configurable-keybindings` | Still planned |

---

## Code Simplification Opportunities

### 1. `syntax.rs` (582 lines, over lint limit)

**Problem:** Only file exceeding 500-line limit. Keyword tables for 8 languages.

**80/20 fix:** Strip to top 4 languages (Rust, Python, JavaScript, Bash). Delete Go, Java, C, C++, SQL tables. Saves ~150 lines, gets under limit.

**Alternative:** Move keyword tables to `syntax/keywords.rs` (80 lines).

### 2. `AppState` (27 fields)

**Problem:** God object. But it's working. 700+ tests pass.

**80/20 fix:** Don't refactor. The code works. AppState composition is R2 at earliest.

### 3. `session.rs` vs `session_jsonl.rs`

**Current:** Two parallel session systems.

**80/20 fix:** Keep `session.rs` (simple JSON, works). Delete `session_jsonl.rs` and `session_manager/`.

### 4. `build.rs` Lint

**Current:** Custom lint in `runie-core/build.rs` enforces 500-line files, 40-line functions.

**Problem:** `syntax.rs` (runie-tui) violates this but isn't checked because build.rs is only in runie-core.

**80/20 fix:** Move lint to a `cargo xtask lint` command or CI check. Build gate prevents incremental development.

---

## Immediate Action Plan (To Ship)

### Phase 1: Delete Dead Code (1 day)
- [ ] Delete `event_bus.rs`, `orchestrator.rs`, `actors/`
- [ ] Delete `session_jsonl.rs`, `session_manager/`
- [ ] Remove `pub mod` declarations from `lib.rs`
- [ ] Fix any import errors in remaining code
- [ ] Verify build still passes

**Lines removed:** ~3,265  
**Risk:** Low — none of this code is called by runtime

### Phase 2: Simplify Tasks (30 min)
- [ ] Delete 6 tasks that track dead code
- [ ] Mark `r1-core-refactor` as done (3/5 ACs complete, remaining are nice-to-have)
- [ ] Reorder R1 by user value

### Phase 3: Implement Keybindings (2-3 days)
- [ ] `r1-config-keybindings` — highest user-value R1 feature
- No actor infrastructure needed; simple file read + keymap dispatch

---

## Summary

| Metric | Before | After Cut |
|--------|--------|-----------|
| runie-core lines | 13,515 | ~10,250 (−24%) |
| MVP done tasks | 34 | 29 (5 were dead code) |
| R1 tasks | 11 | 6 (consolidated) |
| Lint violations | 1 (syntax.rs) | 0 (after keyword trim) |
| Runtime features | Identical | Identical |

**The 80/20 insight:** We built an actor system for a future architecture, but the runtime uses simple tokio async. The actor code is well-tested, well-documented, and completely unused. Delete it without remorse.
