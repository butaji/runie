# Agent Guidelines

Every change needs fast automatic tests (unit + replay). No `sleep()` in tests.

## Architecture Principles

Events-based, single-source-of-truth actors:

- Each state slice is owned by exactly one actor.
- The only change mechanism is events published by the owning actor.
- Handlers, tools, and tests do not mutate another actor's state directly.
- Read-only projections / snapshots are rebuilt from events.
- Every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event). No orphan `tokio::spawn`.

## File Structure

```
crates/
└── runie-core/    # Port of pi-agent-core: agent loop, state, events, tools, queues
lint-check/        # Build-script-style linter enforcing project rules
tasks/             # Implementation plan: index.json + NN-name.md per step
```

## Linter Rules

`lint-check/src/main.rs` enforces, across `crates/runie-core/src/**.rs`:

| Check | Scope |
|-------|-------|
| Magic numbers (>= 1000) | all production code |
| Orphan `tokio::spawn` calls | all production code |

**Magic numbers** — use named constants for buffer sizes, timeouts, and thresholds. Exempt: numbers below 1000, underscore-separated, hex, HTTP/JSON-RPC codes, test code.

**Orphan spawns** — every `tokio::spawn` must be owned by an actor (handle stored in `JoinSet` or actor mailbox).

**Guidelines**: keep files small, functions focused, and complexity low. Split files around ~400 lines and functions around ~60 lines.

## Naming

The crate is named `runie-core` (a port of `@earendil-works/pi-agent-core`).
This recycles the name from a prior wipe; the previous role ("Config, providers,
permissions") is gone and not part of this build.