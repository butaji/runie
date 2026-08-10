# Runie Architecture

Runie is a Rust port of pi-agent-core with a Grok-inspired TUI, limited to
features supplied by the core port. The current plan and architecture findings
live in [`tasks/plan.md`](../tasks/plan.md) and
[`tasks/findings.md`](../tasks/findings.md).

## Layered architecture

```text
┌─────────────────────────────────────────┐
│  UI layer (pure / MVU)                  │
│  - RenderActor: Snapshot → Frame        │
│  - UiActor: facts → Snapshot            │
│  - Input handlers: user action → intent │
├─────────────────────────────────────────┤
│  Domain layer (pure + actors)           │
│  - Actors own state and business rules  │
│  - Intents trigger actor work           │
│  - Facts broadcast state changes        │
├─────────────────────────────────────────┤
│  IO layer (async)                       │
│  - Files, network, subprocesses, OS     │
│  - Results arrive as events             │
└─────────────────────────────────────────┘
```

Rules:

- **IO is async and actor-owned.** Provider streams, tool execution, and
  long-lived workers are owned by their actors.
- **Actors are the single source of truth.** Each mutable state slice lives in exactly one actor.
- **State sync is event-driven.** Handlers emit **intents**; actors consume intents and emit **facts**. The UI projects facts into a read-only `Snapshot`/`AppState`.
- **The UI layer is pure.** Rendering is `draw(&mut Frame, &Snapshot)`.
- **Complexity is hidden behind declarative DSLs.** Commands, keybindings, and dialog actions compose as small flows.

Declarative macros are used for repetitive typed data: command registries,
dialog schemas, keybinding tables, and replay declarations. Actor lifecycles,
provider streaming, error handling, and complex reducers remain explicit Rust
so ownership and async control flow stay reviewable.

The interactive binary owns an asynchronous crossterm `EventStream` worker.
Key events cross a bounded mailbox before reaching `PromptActor` or `UiActor`;
the render cadence never reads the terminal directly. Animation ticks are
separate from input delivery and only request actor-owned animation advances.

`AgentEvent::Reset` is reduced independently by each owning actor. Transient
turn facts are cleared, while configuration facts such as theme and model
caption are preserved. The runtime YAML reset fixture verifies the resulting
multi-actor snapshots without recompilation.

## Crate map

| Crate | Role |
|-------|------|
| `runie-core` | Agent loop, state, events, queues, tools, hooks, provider boundary |
| `runie-tui` | Actor/MVU TUI, Grok-inspired rendering, YAML and visual replay |
| `lint-check` | Build-script-style architecture and source guardrails |

## Runtime

```text
         TUI client      Headless client     ACP/WS client
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │       EventBus         │
                    │  routes intents/facts  │
                    └─────────────────────────┘
                                 │
              Intent events      │      Facts
              ──────────────────►│◄──────────────────
                                 │
      ┌──────────┬───────────────┼───────────────┬──────────┐
      │          │               │               │          │
 AgentState  Queues          Provider/Tool    Loop       TUI actors
   Actor     Actors             Actors        Actor      (MVU)
      │          │               │               │          │
      └──────────┴───────────────┴───────────────┴──────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │  AppState/Snapshot (pure)
                    └─────────────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │     RenderActor (pure)  │
                    └─────────────────────────┘
```

The event bus connects long-lived actors. The TUI is a consumer of facts and
producer of intents; it does not own core state. `watch` snapshots are the
renderer input; compatibility replay adapters are retained only for older
deterministic fixtures and are not live state owners.

## Core concepts

### Events

`CoreEvent` is the single vocabulary for state transitions:

- **Intents** — fire-and-forget requests to an actor. Examples: `SetTheme`, `SubmitInput`, `RunTurn`.
- **Facts** — broadcast state changes produced by actors. Examples: `ConfigLoaded`, `SessionChanged`, `TurnProgress`.

Handlers emit intents. Actors consume intents and emit facts. The UI projects facts into a `Snapshot`. Durable facts are persisted; transient facts are UI-only.

### Core boundaries

Sessions, provider catalogs, OAuth, MCP, ACP, skills, compaction, and browser
proxy behavior are not part of this port unless explicitly promoted into
`runie-core`. See `crates/runie-core/PORT_NOTES.md`.

## Build guardrails

`lint-check` enforces named thresholds and owned task spawning in core
production code. The complete local verification command is `just ci`.

## Testing philosophy

Replay and TUI scenario fixtures are YAML-first; compiled tests are reserved
for concurrency, timing, macro expansion, and serialization boundaries.
