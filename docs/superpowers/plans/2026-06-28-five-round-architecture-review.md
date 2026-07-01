# Five-Round Architecture & Code Review — 2026-06-28

**Date:** 2026-06-28  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Replace custom code with crates/libraries/OS features where it clearly reduces maintenance. Everything must stay events-based with SSOT actors.

This review combines:
- Codebase exploration of `crates/runie-core`, `runie-tui`, `runie-agent`, `runie-provider`.
- Survey of `~/Code/agents/{codex,gptme,openclaw}` recommended patterns.
- `ctx7` documentation lookups for `ractor`, `figment`, `sqlx`, `tui-input`, `rmcp`, `strum`, `pulldown-cmark`.

## Round documents

| Round | Focus | Document |
|-------|-------|----------|
| 1 | SSOT actors & state ownership | [`2026-06-28-round-1-ssot-actors-and-state-ownership.md`](./2026-06-28-round-1-ssot-actors-and-state-ownership.md) |
| 2 | Replace custom code with crates/libs | [`2026-06-28-round-2-replace-custom-code-with-crates.md`](./2026-06-28-round-2-replace-custom-code-with-crates.md) |
| 3 | Actor lifecycle & async-work ownership | [`2026-06-28-round-3-actor-lifecycle-and-async-work.md`](./2026-06-28-round-3-actor-lifecycle-and-async-work.md) |
| 4 | Module boundaries & TUI/DSL | [`2026-06-28-round-4-module-boundaries-and-tui-dsl.md`](./2026-06-28-round-4-module-boundaries-and-tui-dsl.md) |
| 5 | Pareto simplification & integration roadmap | [`2026-06-28-round-5-pareto-integration-roadmap.md`](./2026-06-28-round-5-pareto-integration-roadmap.md) |

## Cross-cutting principles

1. **Single Source of Truth (SSOT):** each runtime fact is owned by exactly one actor.
2. **Events are the change mechanism:** no direct mutation of another actor's state.
3. **No mirrored state:** projections are read-only and rebuilt from events.
4. **Observed async work:** every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event).
5. **Less code:** prefer crates, OS features, and standard patterns over custom implementations.

## Existing context

- ADR: [`2026-07-01-events-based-ssot-actors.md`](./2026-07-01-events-based-ssot-actors.md)
- Task template: `tasks/TEMPLATE.md`
- Testing strategy: `AGENTS.md` (4 layers)

## Task index

New tasks created from this review are listed in `tasks/index.json`. Existing tasks are referenced rather than duplicated. See the per-round documents for the mapping.
