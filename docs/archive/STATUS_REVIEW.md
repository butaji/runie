# Status Review — Tasks, Code & Documentation

**Date:** 2026-06-08  
**Scope:** tasks/, crates/, docs/SPEC.md, AGENTS.md

---

## Executive Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total tests passing | 707 | >0 | ✅ |
| Clippy errors | 0 | 0 | ✅ |
| Build status | Pass | Pass | ✅ |
| Files >500 lines | 1 | 0 | ⚠️ |
| MVP tasks done | 30/35 | 35/35 | 🟡 |
| R1 tasks done | 0/11 | — | 🔵 |
| Tasks with specific test refs | 3/48 | 48/48 | 🔴 |

---

## 1. Code Health

### ✅ What's Working
- **Build passes** across all 5 crates
- **Clippy is clean** — zero errors
- **707 tests pass**: runie-core (446), runie-agent (74), runie-tui (76), runie-term (82), runie-provider (29)
- **update.rs split** — monolith replaced by `update/{mod,input,agent,slash,queue}.rs`
- **O(1) append_response** — `last_assistant_index` cache implemented

### ⚠️ Lint Violations
| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `crates/runie-tui/src/syntax.rs` | 582 | 500 | **Over by 82 lines** |
| `crates/runie-core/src/session_jsonl.rs` | 500 | 500 | At limit |

`syntax.rs` is the only file exceeding the 500-line limit. It is marked `mvp-tui-syntax: done` but violates the project's own linter rule.

### 📊 AppState Field Count
**Current: 27 fields** (was 28; `last_assistant_index` and `thought_seq` were added after the original count).

Still a god object. `r1-core-refactor` correctly identifies this as unfinished, but the task is stalled because splitting AppState requires updating 75+ tests in runie-core.

---

## 2. Task-Code Consistency Issues

### 🔴 mvp-tui-syntax: "done" but over lint limit
- **File:** `syntax.rs` is 582 lines (limit: 500)
- **Impact:** Violates AGENTS.md linter rules
- **Fix:** Either slim `syntax.rs` to <500 lines or update the linter to exempt generated/test-heavy modules

### 🟡 mvp-input-history: "in-progress" scope creep
- **Done:** Up/Down arrow navigation works
- **Not done:** Persistent history across sessions, search/filter
- **Recommendation:** Split into two tasks:
  - `mvp-input-history` → mark **done** (navigation works)
  - `r1-input-history-persistence` → new task for session persistence + search

### 🟡 mvp-input-multiline: confusing status
- **Status text:** `"in-progress (was blocked - Shift+Enter and Ctrl+J handlers implemented)"`
- **Done:** Shift+Enter, Ctrl+J, newline insertion at cursor
- **Not done:** Backspace at line start, cursor positioning across lines
- **Recommendation:** Mark **done** (core multiline works). Create `mvp-input-multiline-cursor` for remaining polish.

### 🟡 mvp-config-hot-reload: still "todo" in MVP
- TOML config parsing is done (`mvp-config-toml: done`)
- File watcher / hot reload is **not implemented**
- **Recommendation:** Move to R1. It's infrastructure, not MVP-critical.

### 🟢 r1-core-refactor: mostly done
- ✅ `update.rs` split into modules
- ✅ Clippy warnings fixed
- ✅ `last_assistant_index` cache (O(1) append)
- ✅ All tests pass
- ❌ AppState composition (27 fields → sub-structs)
- ❌ `VisibleRegion` removal

**Recommendation:** Scope down `r1-core-refactor` to mark the 3 done items, then create a focused `r1-appstate-composition` task for the remaining work.

---

## 3. Documentation-Task Mismatch

### SPEC.md R1 Checklist is Stale
SPEC.md R1 shows unchecked boxes for items already completed:
- `[ ] Split update.rs` — **Done** (update/ module exists)
- `[ ] Fix clippy warnings` — **Done** (clippy clean)
- `[ ] Cache optimizations` — **Done** (`last_assistant_index` exists)

### ADRs are Inconsistent
- `docs/adr/0004-ui-actors-and-rendering.md` — Says UI actor hierarchy is "future design target, not R1 commitment" ✅
- `docs/adr/0009-dedicated-actors-for-cross-cutting-concerns.md` — Lists telemetry/safety/clipboard as **functions**, not actors ✅
- `docs/adr/0010-skills-as-event-interceptors.md` — Deferred to R2/R3 ✅

ADRs were updated correctly. SPEC.md is the one lagging.

---

## 4. Test Documentation Gap

### 28 of 30 Done MVP Tasks Have Zero Specific Test References

Done tasks reference the placeholder `## Tests` section from `TEMPLATE.md` but do **not** list actual test function names.

| Task | Test Refs | Status |
|------|-----------|--------|
| `mvp-core-session-persistence` | 14 | ✅ Documented |
| `mvp-tui-diff` | 18 | ✅ Documented |
| `mvp-tui-syntax` | 13 | ✅ Documented |
| All other 27 done MVP tasks | 0 | 🔴 Placeholder only |

**Problem:** If a task file says "done" but only has placeholder tests, there's no way to verify which tests cover the feature without grepping the entire codebase.

**Root cause:** The batch script appended generic `## Tests` sections without filling in actual test names.

---

## 5. Task Structure Recommendations

### Consolidate R1 Actor Tasks
4 R1 tasks are infrastructure stubs with no user-facing value:
- `r1-actor-tool-actors`
- `r1-actor-queue-agent`
- `r1-actor-session-manager`
- `r1-actor-config-agent`

These share a pattern: ActorId exists in `event_bus.rs`, `orchestrator.rs` has spawn methods, but **no actual actor implementations exist**.

**Recommendation:** Merge into `r1-actor-infrastructure` (single task) and prioritize user-facing R1 features:
1. `r1-config-keybindings` — highest user value
2. `r1-tui-bash-prefix` — medium user value
3. `r1-tui-chunk-events` — medium user value

### Defer Low-Value Tasks
| Task | Reason |
|------|--------|
| `r2-queue-delivery-mode` | Default behavior works; configurable modes are premature optimization |
| `r1-tui-render-cleanup` | Only test file organization; no user value |
| `r1-agent-module-split` | Code quality only; runie-agent is already <500 lines per file |

---

## 6. Immediate Actions (Priority Order)

### P0 — Fix Inconsistencies
1. **Update SPEC.md R1** — Check off completed items (update.rs split, clippy, cache)
2. **Fix or exempt `syntax.rs`** — Either slim to <500 lines or document why it's exempt
3. **Move `mvp-config-hot-reload`** from MVP to R1

### P1 — Task Hygiene
4. **Mark `mvp-input-history` done** (navigation works); create `r1-input-history-persistence`
5. **Mark `mvp-input-multiline` done** (core works); create `mvp-input-multiline-cursor`
6. **Scope down `r1-core-refactor`** — Mark done ACs as done; extract `r1-appstate-composition`

### P2 — Documentation
7. **Backfill test references** in done MVP tasks — link actual `#[test]` function names
8. **Consolidate R1 actor stubs** into single `r1-actor-infrastructure` task

---

## Appendix: Full Task Inventory

### MVP (35 tasks)
| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | mvp-core-bus | done | |
| 2 | mvp-core-orchestrator | done | |
| 3 | mvp-core-event-unification | done | |
| 4 | mvp-core-session-persistence | done | 14 test refs |
| 5 | mvp-core-provider-trait | done | |
| 6 | mvp-tools-bash | done | |
| 7 | mvp-tools-read | done | |
| 8 | mvp-tools-write | done | |
| 9 | mvp-tools-edit | done | |
| 10 | mvp-tools-ls | done | |
| 11 | mvp-tools-grep | done | |
| 12 | mvp-tools-find | done | |
| 13 | mvp-tools-truncation | done | |
| 14 | mvp-tui-streaming | done | |
| 15 | mvp-tui-sorting | done | |
| 16 | mvp-tui-footer | done | |
| 17 | mvp-tui-thinking | done | |
| 18 | mvp-tui-collapse | done | |
| 19 | mvp-tui-markdown | done | |
| 20 | mvp-tui-diff | done | 18 test refs |
| 21 | mvp-tui-syntax | done | **582 lines — over limit** |
| 22 | mvp-tui-ansi | done | |
| 23 | mvp-tui-scrollbar | done | |
| 24 | mvp-session-jsonl | done | |
| 25 | mvp-session-list-delete | done | |
| 26 | mvp-session-persistence | done | 14 test refs |
| 27 | mvp-input-slash-commands | done | |
| 28 | mvp-input-queue | done | |
| 29 | mvp-input-file-refs | done | |
| 30 | mvp-input-multiline | in-progress | Should be done |
| 31 | mvp-input-history | in-progress | Should be done |
| 32 | mvp-safety-blacklist | done | |
| 33 | mvp-safety-output-limits | done | |
| 34 | mvp-config-toml | done | |
| 35 | mvp-config-hot-reload | todo | Should move to R1 |

### R1 (11 tasks)
| # | Task | Status | User Value |
|---|------|--------|------------|
| 1 | r1-core-refactor | in-progress | Medium |
| 2 | r1-agent-module-split | todo | Low |
| 3 | r1-tui-render-cleanup | todo | Low |
| 4 | r1-actor-tool-actors | todo | Low (infra) |
| 5 | r1-actor-queue-agent | todo | Low (infra) |
| 6 | r1-actor-session-manager | todo | Low (infra) |
| 7 | r1-actor-config-agent | todo | Low (infra) |
| 8 | r1-config-keybindings | todo | **High** |
| 9 | r1-tui-chunk-events | todo | Medium |
| 10 | r1-tui-collapse-expand | todo | Medium |
| 11 | r1-tui-bash-prefix | todo | Medium |

### R2 (1 task)
| # | Task | Status | Recommendation |
|---|------|--------|----------------|
| 1 | r2-queue-delivery-mode | todo | Defer to R3 |
