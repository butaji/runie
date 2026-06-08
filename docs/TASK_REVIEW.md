# Task & Documentation Review

**Date:** 2026-06-08  
**Scope:** docs/, tasks/, AGENTS.md, SPEC.md, ADRs  
**Principles:** TDD (4-layer testing), 80/20 rule, less code = more value

---

## Findings

### 1. Zero Tasks Reference Tests

**Problem:** Not a single task in `tasks/*.md` mentions tests in its acceptance criteria, despite `AGENTS.md` mandating:
- Layer 1: State/logic (pure functions)
- Layer 2: Event handling (crossterm events)
- Layer 3: Rendering (TestBackend + Buffer)
- Layer 4: Smoke tests (tmux)

**Impact:** Tasks can be marked "done" without automated test coverage. This defeats the TDD workflow.

**Fix:** Add a mandatory `## Tests` section to every task. Update `AGENTS.md` with task authoring guidelines.

---

### 2. R1 Milestone is Over-Engineered

**Problem:** R1 defines ~20 separate actor tasks, many of which describe trivial concerns as full actors:

| Task | Why It's Over-Engineered |
|------|--------------------------|
| `r1-actor-telemetry-agent` | Telemetry = accumulator function, not an actor |
| `r1-actor-safety-agent` | Safety = pure validation function (`runie-agent/src/safety.rs` already exists) |
| `r1-actor-clipboard-agent` | Clipboard = async utility, not a lifecycle-managed actor |
| `r1-actor-file-lookup` | @-file resolution = async function |
| `r1-actor-command-agent` | Slash commands = parser in `update/slash.rs`, already works |
| `r1-actor-skills` | Skills system is R2/R3 scope; interceptor architecture is premature |
| `r1-ui-root` | UI already routes via render loop; 5 separate UI actors adds boilerplate |
| `r1-ui-input-agent` | Input handling is already modular in `update/input.rs` |
| `r1-ui-scroll-agent` | Scroll state is 3 fields; doesn't need its own actor |
| `r1-ui-chat-agent` | Chat elements are a data transform, not an actor |
| `r1-ui-popup-agent` | Hints are computed inline; no async lifecycle |
| `r1-actor-unified-event` | Duplicate of `mvp-core-event-unification` (already done) |

**The 80/20 alternative:** The working codebase already has 477 tests and a working event loop. R1 should fix **actual problems** from `REFACTOR_PLAN.md` and `REVIEW.md`:
- Split `update.rs` (623 lines) into modules
- Split `AppState` (28 fields) into composed structs
- Fix clippy warnings
- Add **user-facing** features: keybindings, `!command`, per-chunk events

**Fix:** Consolidate R1 from 21 tasks to ~10. Remove actor-for-every-concern tasks. Update `SPEC.md` and ADRs to match.

---

### 3. MVP Tasks Are Fine-Grained but Untested

**Problem:** 34 MVP tasks cover every tool and TUI feature individually. This is good for tracking, but none require tests.

**Fix:** Batch-add `## Tests` sections to all MVP tasks. Keep granularity.

---

### 4. REFACTOR_PLAN.md Problems Are Not Reflected in Tasks

**Problem:** `REFACTOR_PLAN.md` identifies P0 issues:
- `update.rs` = 623 lines (lint violation)
- `AppState` = 28 fields (god object)
- `finish_turn()` = 8 unrelated steps
- `append_response` = O(n) per chunk
- Clippy warnings in prod

None of these have dedicated tasks. They should be in R1.

**Fix:** Create `r1-core-refactor` task covering the top issues from REFACTOR_PLAN.

---

### 5. R2 `queue-delivery-mode` Is Premature Optimization

**Problem:** Configurable delivery modes (`one-at-a-time` vs `all`) add complexity for marginal UX gain. The default behavior already works.

**Fix:** Move to R3 or keep as low-priority. Not essential for 80/20.

---

### 6. SPEC.md R1 Describes Fantasy Architecture

**Problem:** `SPEC.md` R1 section shows an elaborate actor hierarchy that doesn't match the working code and isn't justified by user value.

**Fix:** Rewrite R1 to describe code-quality improvements + high-value user features.

---

## Recommendations Summary

| Action | Files |
|--------|-------|
| Add task authoring guidelines | `AGENTS.md` |
| Create task template | `tasks/TEMPLATE.md` |
| Add `## Tests` to all tasks | `tasks/*.md` |
| Consolidate R1 tasks | `tasks/index.json`, `tasks/r1-*.md` |
| Update SPEC.md R1 | `docs/SPEC.md` |
| Update ADRs 0009, 0010 | `docs/adr/` |

## New R1 Task List (Consolidated)

1. `r1-core-refactor` — Split update.rs, compose AppState, fix clippy, cache optimizations
2. `r1-agent-module-split` — Split runie-agent lib.rs into modules
3. `r1-tui-render-cleanup` — Split render tests, fix lint violations
4. `r1-actor-tool-actors` — Spawn per-invocation tool actors (keep, already partially implemented)
5. `r1-actor-queue-agent` — Message queue actor (keep, already has ActorId)
6. `r1-actor-session-manager` — Session persistence actor (keep, already has ActorId)
7. `r1-actor-config-agent` — Config watcher actor (keep, already has ActorId)
8. `r1-config-keybindings` — Configurable keybindings (high user value)
9. `r1-tui-chunk-events` — Event-per-chunk streaming
10. `r1-tui-bash-prefix` — `!command` support
11. `r1-tui-collapse-expand` — Ctrl+Shift+E collapse/expand feed

Removed: 10 over-engineered actor/UI tasks + duplicate unified-event task.
