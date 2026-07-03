# Fifth-Pass Five-Round Review — SSOT Actor State Machines & Event Protocol

**Goal:** fix the core state-machine bugs that undermine the events-based SSOT actor architecture. This pass is a deep, vertical review of turn lifecycle, event dispatch, actor messaging, and durable replay.

It builds on passes 1–4. Where earlier passes identified symptoms (direct `AppState` mutation, duplicated state), this pass identifies the root cause: **two competing turn state machines** (`runie-agent` facts and `TurnActor` facts) both updating `AppState`.

## Round documents

| Round | Focus | Document |
|-------|-------|----------|
| 1 | TurnState / AgentState SSOT | [`2026-06-28-fifth-pass-round-1-turn-state-ssot.md`](./2026-06-28-fifth-pass-round-1-turn-state-ssot.md) |
| 2 | Event dispatch and idempotency | [`2026-06-28-fifth-pass-round-2-dispatch-idempotency.md`](./2026-06-28-fifth-pass-round-2-dispatch-idempotency.md) |
| 3 | Actor message protocols | [`2026-06-28-fifth-pass-round-3-actor-messages.md`](./2026-06-28-fifth-pass-round-3-actor-messages.md) |
| 4 | Durable event log and replay | [`2026-06-28-fifth-pass-round-4-durable-replay.md`](./2026-06-28-fifth-pass-round-4-durable-replay.md) |
| 5 | Pre-implementation roadmap | [`2026-06-28-fifth-pass-round-5-execution-roadmap.md`](./2026-06-28-fifth-pass-round-5-execution-roadmap.md) |

## Bottom-line recommendation

1. `TurnActor` becomes the sole owner of turn/queue/token/inflight state.
2. `runie-agent` sends raw inputs to `TurnActor`; `TurnActor` emits the canonical facts.
3. `AppState` projects only from `TurnActor` events.
4. Every state change has a corresponding fact; every projection handler is idempotent.
5. Derived values (speed, duration, compaction) are computed in projection code, not carried in events.

## Sources

- Deep codebase exploration of `runie-core/actors/turn`, `model/state`, `update/`, `event/`.
- Survey of `~/Code/agents/{flue,omegacode,langgraph,kimi-code,openclaw,jcode}`.
- `ctx7` for `ractor`, `serde`, `tokio` patterns (consulted in earlier passes).
