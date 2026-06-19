# Runie Architecture

Runie is a terminal-native harness for LLM-powered coding agents. It is not a chat website and not tied to one provider: it is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

This document describes the high-level architecture. The code and tests are written as small, declarative DSLs so that the details stay self-explaining.

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

```
┌─────────────────────────────────────────────────────────────┐
│                   EventBus<CoreEvent>                        │
│      (tokio broadcast + bounded replay buffer)               │
└─────────────────────────────────────────────────────────────┘
       ▲      ▲        ▲           ▲              ▲
       │      │        │           │              │
┌──────┴──┐ ┌─┴───────┐ ┌────┴─────┐ ┌───────┴───┐ ┌───────┴──┐
│ Input   │ │ Agent   │ │ Config   │ │ Session  │ │   UI     │
│ Actor   │ │ Actor   │ │ Actor    │ │ Actor    │ │  Actor   │
└─────────┘ └────┬────┘ └──────────┘ └───────────┘ └────┬─────┘
                 │ spawns                                │
                 ▼                                       │
          ┌──────────────┐                         Snapshot
          │ Subagents /  │                               │
          │ Tool calls   │                               ▼
          └──────────────┘                         ┌───────────┐
                                                   │ Render    │
                                                   │ Actor     │
                                                   └───────────┘
```

Actors are plain `tokio` tasks. They publish and subscribe to a typed `EventBus`. State is owned by the actors, not by a central loop.

- `InputActor` reads crossterm events and publishes `InputEvent`s.
- `AgentActor` runs the LLM turn loop, publishes streaming deltas, tool calls, and turn lifecycle events.
- `SessionActor` persists durable events append-only and replays them on load.
- `UiActor` projects events into `AppState` and sends snapshots to the render actor over a `watch` channel.
- `ConfigActor` watches `~/.runie/config.toml` and publishes reload events.

## Core concepts

### Events

`CoreEvent` is the single vocabulary for state transitions. Events are immutable. Durable events are persisted to the session store; transient events are UI-only.

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
  event.rs          # CoreEvent enum and variants
  state.rs          # AppState + sub-states
  session.rs        # Session types
  commands/         # CommandRegistry and slash handlers
  dialog/           # Panel/Form DSL
  harness_skills/   # Skill trait and implementations
  update/           # Event dispatch

crates/runie-agent/src/
  turn.rs           # Agent turn loop
  tools.rs          # Built-in registry assembly
  parser.rs         # Tool-call parsing
  subagent.rs       # Subagent runner

crates/runie-engine/src/
  tool/             # Built-in tool implementations

crates/runie-tui/src/
  main.rs           # Entry point and event loop
  ui.rs             # draw_snapshot
  popups/           # Dialog rendering
  theme.rs          # Theme tokens

crates/runie-provider/src/
  openai.rs         # OpenAI-compatible providers
  anthropic.rs      # Anthropic
  minimax.rs        # MiniMax-specific streaming
  model.rs          # Model catalog and traits
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

User settings live in `~/.runie/config.toml`:

```toml
provider = "anthropic"
model = "claude-sonnet-4-6"

[models]
scoped = ["claude-sonnet-4-6", "gpt-4o", "deepseek-chat"]

[truncation]
max_lines = 2000
max_bytes = 51200

[telemetry]
enabled = false
```

## Build guardrails

`crates/runie-core/build.rs` enforces structural limits on production code:

| Metric | Limit |
|--------|-------|
| File lines | 500 |
| Function lines | 40 |
| Approximate complexity | 10 |

Tests are exempt from function-length and complexity checks so they can stay comprehensive.

## Testing philosophy

Tests are written as declarative DSLs rather than shell scripts.

- **Layer 1 — State/logic**: pure functions on `AppState` and domain types.
- **Layer 2 — Event handling**: feed `crossterm` events into handlers and assert events emitted.
- **Layer 3 — Rendering**: `TestBackend` + `Buffer` assertions.
- **Layer 4 — Provider replay / mock-tool E2E**: replay captured SSE streams and inject mock tool outputs so the agent turn runs without real network or shell IO.

There are no tmux or shell-based tests. Every feature must be verifiable with `cargo test`.
