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
| `runie-agent` | Agent turn loop, tool-call parsing, truncation, subagent runner, built-in tools |
| `runie-provider` | LLM provider clients and model catalog (OpenAI-compatible, Anthropic, MiniMax, …) |
| `runie-tui` | CLI entry, Ratatui rendering, panels/forms, theme, terminal setup |
| `runie-server` | RPC / server mode binary |
| `runie-protocol` | Shared IPC types |
| `runie-testing` | Test fixtures, mock providers, and harness helpers |
| `runie-macros` | Derive macros for commands, policies, and events |

## Runtime

```text
         TUI client      Headless client     ACP/WS client
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │      LeaderActor        │  owns the event bus,
                    │  (session, plan, turn,  │  runtime lifecycle,
                    │   MCP, permissions)     │  and durable state
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
                    │     RenderActor (pure)  │  draw(&mut Frame, &Snapshot)
                    └─────────────────────────┘
```

The runtime is centered on a `LeaderActor` that owns the event bus and the long-lived actors. Clients (TUI, headless, ACP, WebSocket) are thin producers of intents and consumers of facts. They do not duplicate runtime logic. This makes it cheap to add new surfaces: a new client only needs to speak the intent/fact protocol.

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

### Tool model (MCP-first)

All tools are exposed through the Model Context Protocol (MCP). Tool input schemas are derived from Rust structs via `schemars`; execution is handled by the MCP runtime. The legacy `Tool` trait, `ToolRegistry`, and text/markup/inline-JSON tool parsers are deprecated.

Execution flow:

1. Provider emits a native `ToolCallStart` + input deltas.
2. Agent forwards the call to the MCP runtime.
3. Permission policy (an MCP interceptor) decides `Allow`, `Ask`, or `Deny`.
4. MCP runtime executes the tool.
5. Result is emitted as a durable `ToolResult` event.

For text-only providers, a thin shim converts tool calls to/from plain text rather than maintaining a permanent parser stack.

### Harness skills

Skills are default-on, configurable interceptors on the agent turn. They register hooks (`on_turn_start`, `on_tool_call`, `on_turn_end`) to implement cross-cutting harness behavior without changing the base model.

Skills are declared in markdown files with YAML frontmatter:

```markdown
---
name: check-work
description: Verify changes with a subagent.
metadata:
  short-description: "Verify changes with a subagent"
triggers:
  - command: /check-work
  - command: /verify
---
```

The generic loader parses frontmatter and emits `SkillLoaded`. Adding a new skill means adding a file, not editing the engine.

Current built-in skills:

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
- **Plan-first** (R4): `PlanActor` owns a graph of proposed steps. The agent emits `PlanCreated`; write tools are blocked until the user emits `ApprovePlan`. `TurnActor` executes approved steps and emits `PlanStepCompleted` facts.
- **Team** (R4): `OrchestratorActor` designs a workflow of roles and routes steps to subagents.

Team mode uses the Orchestrator-Harness Protocol (OHP): a typed plan with roles, sequential/parallel steps, and model-trait preferences. The orchestrator resolves traits to concrete models via the catalog.

## External interfaces

Runie exposes its event bus to the outside world through thin clients that talk to the `LeaderActor`:

1. **ACP over stdio** — JSON-RPC adapter (`runie agent stdio`).
2. **Streaming JSON headless mode** — `runie -p "task"` emits newline-delimited facts.
3. **WebSocket server** — `runie agent serve` for IDE/editor integrations.

All clients send intents and receive the same fact stream. The TUI is just one consumer. Example headless event:

```json
{"type":"text","data":"Hello, "}
{"type":"tool_call_start","id":"call_1","name":"bash"}
{"type":"tool_call_end","id":"call_1"}
{"type":"end","stopReason":"EndTurn","sessionId":"...","requestId":"..."}
```

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

## Declarative configuration

Most runtime behavior is declared in files rather than Rust code. The leader loads these at startup and publishes them as facts.

### Subagent type

```markdown
# resources/agents/explore.md
---
name: explore
description: Fast codebase exploration.
prompt_mode: full
model: inherit
permission_mode: default
agents_md: true
---

You are an expert explorer. Search broadly, then narrow down.
```

### Role / persona

```toml
# resources/roles/implementer.toml
name = "implementer"
description = "Implements the planned changes."
model = "fast"
effort = "high"
system_prompt = "You write clean, tested code."
allowed_tools = ["read_file", "write_file", "bash"]
```

### Model metadata

```yaml
# resources/models/grok-build.yaml
id: grok-build
name: Grok Build
base_url: https://api.x.ai/v1
context_window: 512000
api_backend: responses
supports_backend_search: true
auto_compact_threshold_percent: 80
```

### Permission rule

```toml
[[permissions]]
action = "allow"
tool = "read_file"

[[permissions]]
action = "ask"
tool = "bash"
pattern = "git push"
```

### MCP server

```bash
runie mcp add filesystem npx -y @modelcontextprotocol/server-filesroot ~/Code --transport stdio
```

A generic loader parses frontmatter and emits `SkillLoaded`, `CommandRegistered`, `AgentTypeRegistered`, `PermissionRulesLoaded`, `McpServerLoaded`, etc. Adding a feature usually means adding a file, not editing the engine.

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

## Current cleanup roadmap

The 2026-06-28 architecture and code review found that the implementation has drifted from the documented three-layer model. The highest-priority work is tracked in `tasks/` and summarized in [`docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md`](superpowers/plans/2026-06-28-runie-cleanup-roadmap.md).

### Immediate blockers (P0)

1. **Repair the dialog module** (`tasks/repair-and-canonicalize-dialog-module.md`) — `cargo check --workspace` is failing because `crate::dialog` is no longer declared in `runie-core/src/lib.rs`. The duplicate `runie-tui/src/dialog/` subtree must be removed and the TUI must import dialog types from `runie_core`.
2. **Delete empty facade crates** (`tasks/delete-empty-runie-domain-and-runie-io-crates.md`) — `runie-domain` and `runie-io` are currently empty re-export shells.
3. **Collapse the event taxonomy** (`tasks/collapse-event-intent-kind-taxonomies.md`) — `Event`/`Intent`/`EventKind` are near-mirrors maintained by hand; unify them once the actor runtime is stable.

### High-impact simplification (P1)

4. **Prune dead provider code** (`tasks/prune-dead-provider-code-and-rig-core-dependency.md`) — remove the unused `catalog/`, `registry/`, and `rig_adapter.rs` modules and drop the `rig-core` dependency.
5. **Consolidate the actor runtime on `ractor`** (`tasks/consolidate-actor-runtime-on-ractor.md`) — delete the custom `Actor` trait and actors that are only spawned in tests.
6. **Centralize runtime bootstrap** (`tasks/centralize-runtime-bootstrap-in-leaderactor.md`) — route the TUI and CLI through `Leader::start` instead of manual spawn code.

### Medium-risk consolidation (P2)

7. **Deduplicate provider registry data** (`tasks/deduplicate-provider-registry-data.md`).
8. **Replace legacy tool parsers with a thin shim** (`tasks/replace-legacy-tool-parsers-with-thin-shim.md`).
9. **Narrow the `runie-core` public API** (`tasks/narrow-runie-core-public-api.md`).
10. **Route CLI config through `ConfigActor`** (`tasks/route-cli-config-through-configactor.md`).
11. **Remove dead IPC/event-shaping abstractions** (`tasks/remove-dead-ipc-event-abstractions.md`).

### Background sweep (P3)

12. **Clean up small duplicates and dead code** (`tasks/cleanup-small-duplicates-and-dead-code.md`) — `DynProvider`, duplicate `now()`, skill hooks, built-in tool registry, TUI test helpers, `#[allow(dead_code)]` items, and manual derives.

### Execution order

Start with the P0 items (the build cannot be verified until the dialog module is repaired). P1 actor-runtime work should land before the event-taxonomy consolidation because routing tables depend on it. P2 and P3 tasks can mostly run in parallel once the build is green.

## Testing philosophy

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the full 4-layer test taxonomy.
