# Runie Architecture

Runie is a terminal-native harness for LLM-powered coding agents. It is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

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

- **IO is async and actor-owned.** Blocking IO runs inside dedicated actors (`ConfigActor`, `SessionActor`, `FffIndexerActor`, `IoActor`, `EnvActor`).
- **Actors are the single source of truth.** Each mutable state slice lives in exactly one actor.
- **State sync is event-driven.** Handlers emit **intents**; actors consume intents and emit **facts**. The UI projects facts into a read-only `Snapshot`/`AppState`.
- **The UI layer is pure.** Rendering is `draw(&mut Frame, &Snapshot)`.
- **Complexity is hidden behind declarative DSLs.** Commands, keybindings, and dialog actions compose as small flows.

## Crate map

| Crate | Role |
|-------|------|
| `runie-core` | Events, `AppState`, sessions, config, commands, dialog DSL, harness skills |
| `runie-agent` | Agent turn loop, tool-call parsing, truncation, subagent runner, built-in tools |
| `runie-provider` | LLM provider clients and model catalog |
| `runie-tui` | TUI entry, Ratatui rendering, panels/forms, theme, terminal setup |
| `runie-cli` | CLI entry, headless/print/server modes |
| `runie-testing` | Test fixtures, mock providers, and harness helpers |

## Runtime

```text
         TUI client      Headless client     ACP/WS client
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │      LeaderActor        │
                    │  owns the event bus,    │
                    │  runtime lifecycle,     │
                    │  and durable state      │
                    └─────────────────────────┘
                                 │
              Intent events      │      Facts
              ──────────────────►│◄──────────────────
                                 │
      ┌──────────┬───────────────┼───────────────┬──────────┐
      │          │               │               │          │
   Config    Session           Turn            Input      View
   Actor     Actor             Actor           Actor      Actor
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

The `LeaderActor` owns the event bus and long-lived actors. Clients (TUI, headless, ACP, WebSocket) are thin producers of intents and consumers of facts.

## Core concepts

### Events

`CoreEvent` is the single vocabulary for state transitions:

- **Intents** — fire-and-forget requests to an actor. Examples: `SetTheme`, `SubmitInput`, `RunTurn`.
- **Facts** — broadcast state changes produced by actors. Examples: `ConfigLoaded`, `SessionChanged`, `TurnProgress`.

Handlers emit intents. Actors consume intents and emit facts. The UI projects facts into a `Snapshot`. Durable facts are persisted; transient facts are UI-only.

### Sessions

A session is a persisted sequence of durable events. Loading a session replays those events into the actors. Branches and forks are represented by replaying up to a point and then continuing from there.

### Commands

Slash commands (`/model`, `/save`, `/compact`, …) are registered in a typed `CommandRegistry`. Each command defines a form or direct handler and emits events.

### Tools

Tools are exposed through MCP. Tool input schemas come from `schemars`; execution goes through the MCP runtime. A permission interceptor decides `Allow`, `Ask`, or `Deny`. Results are emitted as durable `ToolResult` events.

### Harness skills

Skills are default-on interceptors on the agent turn. They register hooks (`on_turn_start`, `on_tool_call`, `on_turn_end`) to implement cross-cutting behavior without changing the base model. Skills are declared in markdown files with YAML frontmatter and loaded generically.

Built-in skills: `HashlineEditSkill`, `VerificationLoopSkill`, `StartupContextSkill`, `LoopDetectorSkill`, `ToolSchemaEnricherSkill`.

## Execution modes

- **Solo** (default): user prompt goes directly to `AgentActor` with the session model.
- **Plan-first** (R4): `PlanActor` owns a graph of proposed steps; write tools are blocked until the user approves.
- **Team** (R4): `OrchestratorActor` designs a workflow of roles and routes steps to subagents.

## External interfaces

Thin clients talk to the `LeaderActor`:

1. **Streaming JSON headless** — `runie -p "task"` emits newline-delimited facts.
2. **JSON-RPC server** — `runie server` for IDE integrations.
3. **WebSocket server** — `runie agent serve` for editor integrations.

All clients send intents and receive the same fact stream. The TUI is just one consumer.

## Provider normalization

All providers emit a provider-agnostic `LLMEvent` stream (`TextDelta`, `ThinkingDelta`, `ToolCallStart`, `ToolCallEnd`, `Error`, `Usage`, `Finish`). Provider-specific parsing is isolated in `runie-provider`.

## Async IO discipline

Synchronous file or process IO must never run directly on an async task:

- Prefer `tokio::fs`, `tokio::process`, and async `reqwest`.
- Long-lived storage belongs in its own actor.
- Legacy sync call sites can wrap blocking work with `block_in_place_if_runtime` or `run_blocking_if_runtime` from `crates/runie-core/src/async_io.rs`.

## Config durability

`~/.runie/config.toml` is the single source of truth for credentials, default model, keybindings, and preferences. `ConfigActor` is the only production code that reads or writes this file.

Rules:

- Config mutations are sent to `ConfigActor` as intents.
- `ConfigActor` performs atomic load → mutate → save and publishes `ConfigLoaded`.
- `AppState` updates its config projection only in response to `ConfigLoaded`.
- No handler, command, dialog, or login flow writes the config file directly.

## Build guardrails

`crates/runie-core/build.rs` enforces AppState access, magic-number, and orphan-spawn guardrails on all workspace production code.

## Testing philosophy

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the 4-layer test taxonomy.
