# Runie Architecture

Runie is a terminal-native harness for LLM-powered coding agents. It is not a chat website and not tied to one provider: it is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

This document describes the high-level architecture. The code and tests are written as small, declarative DSLs so that the details stay self-explaining.

## Layered architecture

Runie is split into three layers:

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

- **IO is async and actor-owned.** Blocking or long-lived IO runs inside dedicated actors (`ConfigActor`, `SessionActor`, `FffIndexerActor`, `IoActor`, `EnvActor`). The rest of the app sees only events.
- **Actors are the single source of truth.** Each mutable state slice lives in exactly one actor. No handler, command, or dialog mutates state directly.
- **State synchronization is event-driven.** Handlers emit **intents** (requests). Actors consume intents, update their authoritative state, and publish **facts** (state changes). The UI layer projects facts into a read-only `Snapshot`/`AppState`.
- **The UI layer is pure.** Rendering is a pure function `draw(&mut Frame, &Snapshot)`. View logic is a pure projection of facts.
- **Complexity is hidden behind declarative DSLs.** Commands, keybindings, and dialog actions compose as small flows: `on(trigger).intent(...).then(...)`.

## Crate map

| Crate | Role |
|-------|------|
| `runie-core` | Events, `AppState`, sessions, config, commands, dialog DSL, harness skills |
| `runie-agent` | Agent turn loop, tool-call parsing, truncation, subagent runner |
| `runie-engine` | Concrete built-in tool implementations (`read`, `write`, `edit`, `bash`, search, …) |
| `runie-provider` | LLM provider clients and model catalog (OpenAI-compatible, Anthropic, MiniMax, …) |
| `runie-tui` | CLI entry, Ratatui rendering, panels/forms, theme, terminal setup |
| `runie-print` | Non-interactive print mode binary |
| `runie-json` | Non-interactive JSON mode binary |
| `runie-server` | RPC / server mode binary |
| `runie-protocol` | Shared IPC types |
| `runie-testing` | Test fixtures, mock providers, and harness helpers |

## Runtime

```text
                    User input / crossterm
                            │
                            ▼
              ┌─────────────────────────────┐
              │   Input / command handlers  │  (pure: build intents)
              └─────────────────────────────┘
                            │
              Intent events │ Facts
              ─────────────►│◄─────────────
                            │
      ┌──────────┬──────────┼──────────┬──────────┬──────────┐
      │          │          │          │          │          │
   Config    Session     Turn       Input       View    Notification
   Actor     Actor      Actor      Actor       Actor      Actor
      │          │          │          │          │          │
      └──────────┴──────────┴──────────┴──────────┴──────────┘
                            │
                            ▼
              ┌─────────────────────────────┐
              │  AppState projection (pure) │  reads facts only
              └─────────────────────────────┘
                            │
                            ▼
              ┌─────────────────────────────┐
              │     RenderActor (pure)      │  draw(&mut Frame, &Snapshot)
              └─────────────────────────────┘
```

Actors are plain `tokio` tasks. Each actor owns a slice of authoritative state and communicates through typed intents and facts. There is no central mutable `AppState`; `AppState` is a read-only projection updated by facts.

### Bootstrap and rendering rules

- All startup file I/O (git detection, trust, skills, auth tokens, theme, config) runs inside dedicated actors before the UI event loop starts.
- `AppState` is built by applying facts. It never reads files, blocks, or mutates itself outside the projection path.
- The render path is a pure function `draw(&mut Frame, &Snapshot)`. It never mutates state.
- `InputActor` reads crossterm events and publishes typed input events. Handlers turn those into intents.
- `TurnActor` owns the LLM turn lifecycle, scheduling queues, and token accounting. It consumes streaming events from `AgentActor`/`ProviderActor` and emits session/tool facts.
- `SessionActor` owns the in-memory session and persists durable events append-only.
- `ViewActor` owns derived view/cache state and invalidates the render path.
- `ConfigActor` is the single owner of `~/.runie/config.toml`; it loads, saves, and publishes `ConfigLoaded` facts.
- `ProviderActor` is the single owner of `DynProvider` construction and API-key validation; it resolves credentials through the config actor.

## Core concepts

### Events

`CoreEvent` is the single vocabulary for state transitions. Events are immutable and split into two families:

- **Intents** — fire-and-forget requests to an actor. They describe *what the user or system wants*. Examples: `SetTheme`, `SubmitInput`, `AskPermission`, `RunTurn`.
- **Facts** — broadcast state changes produced by actors. They describe *what changed*. Examples: `ConfigLoaded`, `SessionChanged`, `TurnProgress`, `PermissionResolved`.

Handlers, commands, and keybindings emit intents. Actors consume intents, update their authoritative state, and emit facts. The UI layer projects facts into a read-only `Snapshot`. Durable facts are persisted to the session store; transient facts are UI-only.

### Sessions

A session is a persisted sequence of durable events. Loading a session replays those events into the actors. Branches and forks are represented by replaying up to a point and then continuing from there.

### Commands

Slash commands (`/model`, `/save`, `/compact`, …) are registered in a typed `CommandRegistry`. Each command defines a form or direct handler and emits events.

### Tool model

Tools implement a shared `Tool` trait. The built-in tools live in `runie-engine`; the registry and shared types live in `runie-core` so new tools can be added without depending on the engine.

Execution flow:

1. Provider emits `ToolCallStart` + `ToolCallInputDelta`.
2. Agent parses and validates the input.
3. Permission policy decides `Allow`, `Ask`, or `Deny`.
4. `runie-engine` executes the tool.
5. Result is emitted as a durable `ToolResult` event.

### Harness skills

Skills are default-on, configurable interceptors on the agent turn. They register hooks (`on_turn_start`, `on_tool_call`, `on_turn_end`) to implement cross-cutting harness behavior without changing the base model.

Current skills:

- `HashlineEditSkill` — line-addressed edits with content-hash verification.
- `VerificationLoopSkill` — runs a verification command after the model claims completion.
- `StartupContextSkill` — discovers cwd, tools, and environment before the turn.
- `LoopDetectorSkill` — detects repeated failed tool patterns.
- `ToolSchemaEnricherSkill` — adds examples to tool schemas.

See `crates/runie-core/src/harness_skills/mod.rs` for the trait and hook types.

### Search backend (`fff-search`)

File and content search are backed by the native `fff-search` crate. A long-lived `FffIndexerActor` keeps the index, frecency tracker, and query tracker in memory and serves both agent tools and the TUI `@` picker.

Query syntax:

| Syntax | Example | Effect |
|--------|---------|--------|
| Fuzzy text | `mylib` | Typo-tolerant filename/content search |
| Glob | `*.rs` | Filter by extension or pattern |
| Negation | `!test/` | Exclude matching files |
| Git filter | `git:modified` | Show files with given git status |
| Location | `lib.rs:42:5` | Jump to line/column |
| Quoted | `"exact phrase"` | Match exact phrase in content |

## Execution modes

- **Solo** (default): the user prompt goes directly to `AgentActor` with the session model.
- **Team** (R4): `OrchestratorActor` designs a workflow of roles and routes steps to subagents.

Team mode uses the Orchestrator-Harness Protocol (OHP): a typed plan with roles, sequential/parallel steps, and model-trait preferences. The orchestrator resolves traits to concrete models via the catalog.

## Provider normalization

All providers emit a provider-agnostic `LLMEvent` stream:

```rust
pub enum LLMEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallInputDelta { id: String, delta: String },
    ToolCallEnd { id: String },
    Error(LLMError),
    Usage { input_tokens: u32, output_tokens: u32 },
    Finish,
}
```

Provider-specific parsing (for example MiniMax XML tool-call delimiters) is isolated in `runie-provider`.

## Code layout

```
crates/runie-core/src/
  event/            # CoreEvent enum and variants
  model/state/      # AppState + sub-states
  session.rs        # Session types
  actors/           # ConfigActor, ProviderActor, SessionActor, actor trait
  commands/         # CommandRegistry and slash handlers
  dialog/           # Panel/Form DSL
  harness_skills/   # Skill trait and implementations
  view/             # Element, Feed, LazyCache (domain projection)
  update/           # Event dispatch

crates/runie-agent/src/
  actor.rs          # AgentActor (interactive turn executor)
  turn.rs           # Agent turn loop
  subagent.rs       # Subagent runner

crates/runie-engine/src/
  tool/             # Built-in tool implementations

crates/runie-tui/src/
  main.rs           # Entry point and event loop
  ui/               # Rendering (Ratatui widgets, draw_snapshot)
  core_ui/          # Re-exports from runie-core::view
  popups/           # Dialog rendering
  theme/            # Theme tokens

crates/runie-provider/src/
  openai/           # OpenAI-compatible providers
  factory.rs        # Provider construction
  mock.rs           # Mock provider for tests
```

## Crate decisions

We prefer maintained crates over custom code when they reduce complexity and improve correctness.

| Area | Choice | Reason |
|------|--------|--------|
| Markdown parsing | `pulldown-cmark` | Standard, well-tested |
| Syntax highlighting | `syntect` | Existing Sublime grammar ecosystem |
| Diff generation | `similar` | Reliable diff output |
| Token counting | `tiktoken-rs` | Matches OpenAI tokenization |
| File search | `fff-search` | Native, typed, frecency-aware |
| Config watcher | `notify` | Removes polling |
| Clipboard | `arboard` | Cross-platform, no temp files |
| Diff parsing | `patch` | Fixes hand-rolled line-number bugs |
| YAML frontmatter | `serde_yml` | Correct YAML parsing |
| Color helpers | `palette` | Color-space correct blending |
| Fuzzy matching | `nucleo-matcher` | Better Unicode/ranking |
| Word wrapping | `textwrap` | Better line breaking |
| Session store | `redb` | ACID + indexing |

Custom code is kept where it is project-specific or where a drop-in crate does not exist: the text-input widget, command-palette DSL, dialog/form DSL, update dispatcher, actor runtime, and streaming stable/tail buffer.

## Configuration

User settings live in `~/.runie/config.toml`. See [Configuration](Configuration.md) for the full reference.

## Async IO discipline

The TUI runs on a multi-threaded Tokio runtime. Synchronous file or process IO must never run directly on an async task.

- Prefer native async APIs: `tokio::fs` for files, `tokio::process` for subprocesses, async `reqwest` for HTTP.
- Long-lived storage belongs in its own actor (for example `FffIndexerActor` and `SessionActor`).
- Legacy call sites that cannot be fully async yet must wrap blocking work with the helpers in `crates/runie-core/src/async_io.rs`:
  - `run_blocking_if_runtime` — fire-and-forget blocking work on a Tokio blocking thread.
  - `block_in_place_if_runtime` — run a short blocking closure off the async runtime and return the result.
- These helpers fall back to synchronous execution when no runtime is present, which keeps unit tests fast and deterministic.

Concrete remediation order:
1. If the call site can be made `async`, use `tokio::fs` / `tokio::process`.
2. If the caller is a sync function reached from an async actor (e.g., the update dispatcher or a sync skill hook), wrap the IO in `block_in_place_if_runtime`.
3. If the work is fire-and-forget or long-running, use `spawn_blocking` or `run_blocking_if_runtime`.

New code should default to async or event-based actors; the helpers are a tactical bridge, not the preferred pattern.

## Config durability

`~/.runie/config.toml` is the single source of truth for provider credentials, default model, keybindings, and preferences. `ConfigActor` is the only production code that reads or writes this file.

Rules:

- All config mutations are sent to `ConfigActor` as intents (`ConfigMsg`).
- `ConfigActor` performs atomic load → mutate → save under a write lock on a blocking thread, then publishes `ConfigLoaded`.
- `AppState` updates its config projection only in response to `ConfigLoaded` facts.
- No handler, command, dialog, or login flow writes the config file directly.
- `login_config.rs` is being removed; its helpers are replaced by `ConfigActor` messages and a `ConfigStore` trait for tests.
- Do not nest `block_in_place` calls: a function that already runs on a blocking thread must not call `block_in_place_if_runtime` again.
- Prefer atomic updates over fire-and-forget background writes for durable state.

## Build guardrails

`crates/runie-core/build.rs` enforces structural limits on production code:

| Metric | Limit |
|--------|-------|
| File lines | 500 |
| Function lines | 40 |
| Approximate complexity | 10 |

Tests are exempt from function-length and complexity checks so they can stay comprehensive.

## Testing philosophy

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the full 4-layer test taxonomy.
