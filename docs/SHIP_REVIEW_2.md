# Ship Review #2 — Architecture & Code Audit

**Date:** 2026-06-08  
**Mandate:** 80/20. Less code, same value. Ship.

---

## Summary

The product works. 707 tests pass. Build is clean. But **~25% of runie-core is dead code** — well-tested, well-documented, and completely unused by the runtime. The SPEC.md and task tracker still reflect a fantasy actor architecture that was never wired into `main.rs`.

### The Real Architecture (main.rs)

```
tokio::spawn(agent_loop(cmd_rx, agent_tx));      // Agent turn execution
tokio::spawn(input_reader(input_tx));             // Crossterm → CoreEvent
tokio::spawn(render_task(terminal, render_rx));   // Snapshot → ratatui draw
event_loop(state, input_rx, agent_rx, cmd_tx, render_tx);  // Single-threaded reducer
```

Three tokio tasks + one event loop. No actors. No EventBus. No Orchestrator.

---

## Dead Code Inventory

### 1. Actor System (~2,300 lines, 17.5% of runie-core)

| File | Lines | Runtime Usage |
|------|-------|---------------|
| `event_bus.rs` | 441 | **Zero** — runtime uses `tokio::sync::mpsc` |
| `orchestrator.rs` | 337 | **Zero** — never instantiated |
| `actors/` | 1,521 | **Zero** — never spawned |
| **Subtotal** | **2,299** | |

The `actors/` directory contains:
- `config_agent.rs` (307 lines) — config watcher actor
- `queue_agent.rs` (336 lines) — message queue actor
- `tool_actor.rs` + `tool_execute.rs` + `tools/` (651 lines) — tool execution actors
- `session_manager.rs` (6 lines) — re-export stub
- `mod.rs` (17 lines)

**None of these are referenced by `main.rs`.** The runtime's agent loop calls `run_agent_turn()` directly. The queue is a `VecDeque` on `AppState`. Config is loaded once at startup. Sessions use `session.rs` (simple JSON).

### 2. Session JSONL + Manager (~966 lines, 7.3% of runie-core)

| File | Lines | Runtime Usage |
|------|-------|---------------|
| `session_jsonl.rs` | 500 | **Zero** — only imported by `session_manager/` |
| `session_manager/` | 466 | **Zero** — runtime uses `session.rs` |
| **Subtotal** | **966** | |

The runtime's slash commands (`/save`, `/load`, `/sessions`, `/delete`) call:
```rust
crate::session::save(name, &session)
crate::session::load(name)
crate::session::list()
crate::session::delete(name)
```

These are defined in `session.rs` (230 lines, simple JSON files). The JSONL event-log system in `session_jsonl.rs` + `session_manager/` is a parallel, unused implementation.

### 3. Dead Field: `render_generation`

```rust
pub render_generation: u64,  // model.rs:55
```

Defined and initialized to 0. **Never read anywhere.** Likely a leftover from a render optimization that was never implemented.

### 4. `dsl` Module (Test-Only)

```rust
#[cfg(test)]
pub mod dsl;
```

Correctly gated behind `#[cfg(test)]`. Not dead, but test-only.

### 5. Total Dead Code

| Category | Lines | % of runie-core |
|----------|-------|-----------------|
| Actor system | 2,299 | 17.5% |
| Session JSONL/manager | 966 | 7.3% |
| **Total** | **3,265** | **24.8%** |

---

## SPEC.md Issues

### R1 Section Shows Cut/Cancelled Work

The SPEC.md R1 checklist still includes:

- `[ ] **Module split**` — Task `r1-agent-module-split` was **cut** (no user value)
- `[ ] **Split render tests**` — Task `r1-tui-render-cleanup` was **cut** (no user value)
- `[ ] **ToolActors**` — Would add actors on dead architecture
- `[ ] **QueueAgent**` — Would add actors on dead architecture
- `[ ] **SessionManager**` — Would add actors on dead architecture
- `[ ] **ConfigAgent**` — Would add actors on dead architecture

These should be removed from SPEC.md. The ConfigAgent is not needed for keybindings — just read a JSON file at startup.

### Missing from SPEC.md

- `[x] **Ctrl+Shift+E**` — Already implemented (main.rs has `ToggleExpand` mapping, tests verify it)
- `[x] **!command**` — Task `r1-tui-bash-prefix` is marked **done**

---

## build.rs Issues

### 1. Lint Only Checks runie-core

```rust
let crates_path = workspace_root.join("crates");  // but only called from runie-core/build.rs
```

The build script lives in `runie-core/build.rs`. It lints `crates/runie-core/src/` and all subdirs. It does **not** check `runie-tui` or other crates. The syntax module was already split into `syntax/` (max file 249 lines), so there are currently no file-size violations in the checked crates.

### 2. Build Gate Prevents Incremental Development

If a file exceeds 500 lines, `cargo build` fails. This means you can't compile while refactoring a file that's temporarily over limit.

**Recommendation:** Move to `cargo xtask lint` or CI check.

---

## Task Tracker Issues

### Tasks That Built Dead Code (Still Marked Done)

| Task | Built | Status | Issue |
|------|-------|--------|-------|
| `mvp-core-bus` | `event_bus.rs` | done | Has warning note; OK |
| `mvp-core-orchestrator` | `orchestrator.rs` | done | Has warning note; OK |
| `mvp-core-event-unification` | Unified Event type | done | **NOT dead** — `event.rs` is core |
| `mvp-session-jsonl` | `session_jsonl.rs` | done | Has warning note; OK |
| `mvp-core-session-persistence` | `session_manager/` | done | Has warning note; OK |

Wait — `mvp-core-event-unification` built the `Event` enum in `event.rs`, which IS used by the runtime. That task is not dead code. The other 4 built dead infrastructure around it.

### MVP Tasks Still Blocking Ship?

| Task | Status | Blocking? | Recommendation |
|------|--------|-----------|----------------|
| `mvp-input-multiline-cursor` | todo | No | Minor polish. Backspace at line start works 90% of time. |
| `mvp-config-hot-reload` | todo | No | Restart is fine for config changes. |

### R1 Tasks — Prioritized for Ship

| Priority | Task | User Value | Effort | Blocking? |
|----------|------|------------|--------|-----------|
| P0 | `r1-config-keybindings` | **High** | Medium | Nothing — just read JSON, dispatch |
| P1 | `r1-tui-chunk-events` | Medium | Low | Nothing |
| P2 | `r1-tui-collapse-expand` | Medium | Low | Nothing |
| P3 | `r1-input-history-persistence` | Low | Low | Nothing |

---

## Recommendations

### Immediate (This Week)

1. **Delete dead code** — Remove `event_bus.rs`, `orchestrator.rs`, `actors/`, `session_jsonl.rs`, `session_manager/`. Remove from `lib.rs`. Fix imports.
2. **Remove `render_generation` field** — 1 line deleted, nothing breaks.
3. **Update SPEC.md R1** — Remove actor tasks, module split, render cleanup. Mark `Ctrl+Shift+E` and `!command` as done.
4. **Move build.rs lint to CI** — Prevent build failures during refactoring.

### Short-Term (Next Sprint)

5. **Implement `r1-config-keybindings`** — Highest user-value remaining feature. No dependencies.
6. ~~**Fix `syntax.rs` to <500 lines**~~ — **Already done.** Split into `syntax/{mod,tokenize,keywords}.rs` (max 249 lines).

### Do Not Do

- Do not implement actor infrastructure (EventBus, Orchestrator, actors)
- Do not split AppState into sub-structs (27 fields is fine, it works)
- Do not add session JSONL/event-log system (JSON files work)
- Do not implement `r2-queue-delivery-mode` (default behavior is fine)

---

## Appendix: Files to Delete

```
crates/runie-core/src/event_bus.rs
crates/runie-core/src/orchestrator.rs
crates/runie-core/src/actors/           (entire directory)
crates/runie-core/src/session_jsonl.rs
crates/runie-core/src/session_manager/  (entire directory)
```

**From `lib.rs`:**
- Remove `pub mod event_bus;`
- Remove `pub mod orchestrator;`
- Remove `pub mod actors;`
- Remove `pub mod session_jsonl;`
- Remove `pub mod session_manager;`
- Remove `session_jsonl` re-exports

**Tests lost:** ~32 (all testing dead code)
**Runtime features lost:** Zero
**Lines removed:** ~3,265
