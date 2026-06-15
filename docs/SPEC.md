# Runie Specification

Terminal coding agent harness in Rust, inspired by [pi](https://pi.dev).

> **Snapshot: 2026-06-14.** For task history, see `tasks/`.
>
> The project is currently executing an R3 simplification pass: unify duplicated
> types, flatten the event system, finish the AppState refactor, and consolidate
> the TUI/term crates. See the [Simplification plan](#simplification-plan-r3).

## Architecture

### Runtime

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                            EventBus<CoreEvent>                                │
│   (tokio::sync::broadcast + bounded replay buffer)                            │
└──────────────────────────────────────────────────────────────────────────────┘
         ▲            ▲             ▲            ▲            ▲
         │            │             │            │            │
┌────────┴───┐ ┌──────┴──────┐ ┌────┴────────┐ ┌┴────────────┐ ┌────────────┐
│ InputActor │ │Orchestrator │ │ ConfigActor │ │SessionActor │ │  UiActor   │
│ (crossterm)│ │Actor        │ │(TOML watch) │ │(JSONL store)│ │(AppState   │
└────────────┘ │(Team mode)  │ └─────────────┘ └─────────────┘ │ projection)│
               └──────┬──────┘                                   └─────┬──────┘
                      │                                                │
                      │ spawns                                         │
                      ▼                                                │
               ┌──────────────┐                                  Snapshot
               │ AgentActor   │                                        │
               │ (Solo +      │                                  ┌──────┴──────┐
               │  subagents)  │                                  │ RenderActor │
               └──────────────┘                                  │  (ratatui)  │
                                                                  └─────────────┘
```

Actors are tokio tasks that publish and subscribe to a typed `EventBus`.
`SessionActor` persists durable events to append-only JSONL files.
`UiActor` projects events into `AppState` and sends snapshots to the render
actor via a `watch` channel. State is owned by the actors/projection actors,
not by a central loop.

In **Team mode**, the `OrchestratorActor` designs and executes multi-agent
workflows. In **Solo mode**, the user prompt goes directly to `AgentActor`.
See `docs/adr/0020-team-mode-orchestration.md` for the design.

### Crates

| Crate            | Role                                                        |
|------------------|-------------------------------------------------------------|
| `runie-core`     | Events, AppState, sessions, config, commands                |
| `runie-agent`    | Tool implementations, agent turn, subagent runner, truncation |
| `runie-provider` | LLM providers, model catalog                                |
| `runie-tui`      | Ratatui rendering, panels/forms, theme                      |
| `runie-term`     | CLI entry, task dispatch, smoke harness                     |
| `runie-print`    | Non-interactive print mode (separate binary)                |
| `runie-json`     | Non-interactive JSON mode                                   |
| `runie-server`   | RPC / server mode                                           |

### Event model

Events flow through a typed `EventBus<CoreEvent>` in `runie-core/src/bus.rs`.
Each actor subscribes to the events it cares about:

- `InputActor` publishes `InputEvent`s.
- `AgentActor` publishes `AgentEvent`s (tool calls, streaming deltas, errors).
- `ConfigActor` publishes `ConfigEvent`s on TOML changes.
- `SessionActor` writes durable events to JSONL and loads sessions by replay.
- `UiActor` subscribes to all events and projects them into `AppState`.

`CoreEvent` is split into durable events (persisted to JSONL) and transient
events (UI-only). The `Event` enum is being flattened and its dispatcher
simplified (`tasks/flatten-event-system.md`).

### Harness Skills

Skills are default-on, configurable interceptors on the event bus (see
`docs/adr/0022-harness-middleware-plugins.md`). They implement harness-level
behaviors that measurably improve agent output without changing the base model:

- **Hashline Edit Skill** — line-addressed edits with content hashes, replacing
  brittle exact-string `search`/`replace`.
- **Verification Loop Skill** — runs a configurable verification command after
  the model claims completion and feeds failures back for a fix pass.
- **Startup Context Injector Skill** — discovers cwd, tools, and environment
  before the turn and injects the result into the system prompt.
- **Loop Detector Skill** — detects repeated failed tool patterns and prompts
  the model to reconsider.
- **Tool Schema Enricher Skill** — adds examples to tool schemas to reduce
  tool-usage failures.

Skills are toggled and configured under `[harness.skills]` in
`~/.runie/config.toml`.

### Search Backend (`fff-search`)

File and content search are backed by the native `fff-search` Rust crate
instead of shelling out to `rg`/`fd`/`find`. A long-lived `FffIndexerActor`
keeps the index, frecency tracker, and query tracker in memory and serves
both agent tools and the TUI `@` picker (see
`docs/adr/0023-fff-search-integration.md`).

Capabilities:

- Unified `search` tool with `mode = files | content | mixed`.
- Typo-resistant fuzzy matching and constraint queries (`*.rs !test/`).
- Frecency ranking based on recent/frequent file access.
- Git-status awareness (`git:modified`, `git:untracked`).
- Definition classifier for `find_definitions`.
- Fast glob and `file:line:col` location parsing.
- Legacy `rg`/`fd` fallback for memory-constrained sessions.

## Features

### Always-on

- 35 LLM providers, ~130 models
- Tool set: `bash`, `read`, `write`, `edit`, `ls`, `grep`, `find`, `fetch_docs`
- Sessions: save/load/list/delete/name/export/import, JSON files in `data_dir`
- Slash commands registered in a typed `CommandRegistry` with form-dialog
  prompts for parameters
- Command palette (Ctrl+P)
- Model selector (Ctrl+L) with recent + provider grouping
- Thinking levels (off/low/medium/high), Shift+Tab to cycle
- Skills (load SKILL.md from user/project dirs)
- Custom prompt templates
- Output truncation (configurable, head/tail strategies)
- Theme system (BUILTIN_THEMES + opaline integration)
- Config hot-reload via polling watcher
- Diagnostics, reload
- TUI features: streaming, markdown, syntax highlight, diff, ANSI, scrollbar,
  input history, undo/redo, multi-line, @-file refs, path completion,
  contextual footer hints, mode suffix in input title, block-level copy (`y`/`Y`)
- Modes: interactive TUI, print, JSON
- `runie-server` crate exists but is not yet a supported RPC surface

### Roadmap (R3)

Planned architecture and UX improvements based on research in `~/Code/agents`:

- **Event-based actor runtime** — tokio-task actors + `EventBus` + JSONL session
  persistence (`tasks/actor-runtime-decision.md`,
  `tasks/event-bus-jsonl-persistence.md`)
- **Harness Skills** — default-on, togglable middleware for edit tools,
  verification loops, context injection, loop detection, and tool-schema
  enrichment (`tasks/harness-skill-*`)
- **Native `fff-search` backend** — unified file/content search and `@` picker
  powered by a long-lived indexer (`tasks/fff-*`)
- **Normalized `LLMEvent` stream** — all providers emit the same event
  vocabulary (`tasks/llm-event-normalization.md`)
- **Model capability flags** — streaming/vision/tools/reasoning/max-tokens per
  model (`tasks/model-capability-flags.md`)
- **`ToolRegistry` trait + MCP client** — built-ins and MCP servers registered
  uniformly (`tasks/tool-registry-trait.md`, `tasks/mcp-client-integration.md`)
- **Permission rulesets** — wildcard allow/ask/deny rules, read-only tool
  classification (`tasks/permission-rulesets.md`)
- **Context compaction** — token-threshold summarization with message metadata
  (`tasks/context-compaction.md`)
- **Streaming stable/tail split** — no tearing during markdown/tool streaming
  (`tasks/streaming-buffer-tail-split.md`)
- **Stateful tool-call rendering** — `Pending/Running/Completed/Error` with
  elapsed time and expand/collapse (`tasks/tool-call-state-rendering.md`)
- **Status indicator widget** — phase + elapsed + interrupt hint
  (`tasks/status-indicator-widget.md`)
- **Semantic theme tokens** — accessible, lintable color system
  (`tasks/semantic-theme-tokens.md`)
- **Session list with summaries** — starred, named, auto-summarized sessions
  (`tasks/session-list-summaries.md`)
- **Crate replacement audit** — evaluate `syntect`, `similar`, `nucleo`,
  `tui-textarea`, `ratatui-markdown`, etc. (`tasks/crate-replacement-audit.md`)

### Roadmap (R4 — Multi-Agent Orchestration)

Solo/Team execution modes based on the design in `docs/adr/0020-team-mode-orchestration.md`:

- **Solo/Team toggle** — per-session UI mode; Solo is the default; `/spawn`
  removed (`tasks/r4-solo-team-mode-toggle.md`)
- **OrchestratorActor** — designs and executes dynamic Team workflows
  (`tasks/r4-orchestrator-actor.md`)
- **Orchestrator-Harness Protocol (OHP)** — typed workflow plan with roles,
  sequential/parallel steps, and model trait preferences
  (`tasks/r4-orchestrator-domain-types.md`)
- **Alignment Q&A** — `ask_user` tool in the Dialog Panel; Autopilot and
  `/plan` command (`tasks/r4-ask-user-tool.md`)
- **One-shot orchestrator LLM** — emits OHP plans after alignment
  (`tasks/r4-one-shot-orchestrator-llm.md`)
- **Model trait resolution** — relative ranking + global priority list + fallback
  chains (`tasks/r4-model-trait-resolution.md`)
- **Isolated subagent sessions** — structured JSON results, tool-policy filtering
  (`tasks/r4-subagent-isolation.md`)
- **Subagent sidebar** — per-agent feeds with `Ctrl+0`..`Ctrl+9` hotkeys
  (`tasks/r4-subagent-sidebar.md`)
- **Team mode integration** — end-to-end Q&A → plan → execute → result
  (`tasks/r4-team-mode-integration.md`)
- **Grok Build TUI parity** — mouse support, contextual hints, richer status bar,
  command palette ranking, `@file` line ranges, theme quantization, welcome
  screen (`tasks/grok-*.md`)

### Simplification plan (R3)

The codebase accumulated duplicated types, a fragmented event system, and an
incomplete state refactor. R3 prioritizes consolidation before adding new
features:

| Priority | Task | Goal |
|----------|------|------|
| P0 | `tasks/unify-config-types.md` | One config TOML schema |
| P0 | `tasks/unify-message-types.md` | One `ChatMessage`/`Role` type |
| P0 | `tasks/unify-tool-result-types.md` | One tool-result type |
| P1 | `tasks/flatten-event-system.md` | Flat `Event` enum, generated name mapping |
| P1 | `tasks/complete-appstate-refactor.md` | Finish sub-state migration |
| P1 | `tasks/coalesce-update-modules.md` | Merge 27 update modules by domain |
| P1 | `tasks/unify-command-dsl.md` | One command definition + execution path |
| P1 | `tasks/merge-runie-term-into-tui.md` | Single TUI crate |
| P1 | `tasks/unify-diff-model.md` | Shared diff type |
| P1 | `tasks/adopt-or-remove-actor-framework.md` | Resolve EventBus/Actor dead code |
| P1 | `tasks/unify-rendering-pipeline.md` | Core AST + TUI renderer only |
| P2 | `tasks/unify-markdown-pipeline.md` | Single markdown pass |
| P2 | `tasks/cleanup-state-helpers.md` | Remove duplicated helpers and dead code |

Historical design documents have been removed; current decisions are captured in ADRs and tasks.

### Test coverage

- 1,794 automated tests listed across the workspace; 1,716 pass and 78 are
  intentionally ignored (e2e / platform-specific)
- 4-layer TDD per `AGENTS.md`: state/logic, event handling, rendering, smoke
- Build-time lint guardrails are 2000 lines/file, 150 lines/function,
  complexity 30 (long-term targets remain 500/40/10; see `AGENTS.md`)
- Pre-existing failures: none blocking the main suite

### Out of scope (by design)

- **Plugins/extensions** — adds complexity without daily-use value
- **OAuth login flow** — API keys in config.toml suffice
- **General DAG workflows with cycles** — Team mode uses sequential + parallel
  groups; arbitrary cycles are out of scope
- **Session tree / branching UI** — `/fork` exists, but no visual tree
- **Custom syntax-highlighting languages** — limited built-in tokenizers; full grammar support via crate audit (`tasks/crate-replacement-audit.md`)
- **Web UI / VS Code extension** — terminal-only

## Code organization

```
crates/
├── runie-core/src/
│   ├── event.rs          # Event enum (all state transitions)
│   ├── model.rs          # ChatMessage, Role, model helpers
│   ├── state.rs          # AppState + sub-state structs
│   ├── session.rs        # Session types + JSON persistence
│   ├── snapshot.rs       # View projection
│   ├── config_reload.rs  # Config watcher + reload logic
│   ├── commands/         # CommandRegistry + handlers/
│   ├── dialog/           # Panel/Form DSL + PanelStack
│   ├── update/           # Event dispatch (input, agent, dialog, ...)
│   ├── skills/           # SKILL.md loading
│   ├── prompts/          # Prompt templates
│   └── (orphaned)        # actor.rs, bus.rs, config.rs, llm_event.rs,
│                         #   mcp.rs, session_actor.rs, session_store.rs,
│                         #   streaming_buffer.rs, tool.rs — not wired to lib.rs
├── runie-agent/src/
│   ├── tools/            # One module per built-in Tool impl
│   ├── tools.rs          # Built-in tool registry assembly
│   ├── turn.rs           # Agent turn loop
│   ├── subagent.rs       # Isolated nested turn for subagents
│   ├── truncate.rs       # TruncationConfig (TOML) + policies
│   ├── accumulator.rs    # Bounded buffer for streaming
│   ├── mutation_queue.rs # Serialized file edits
│   ├── safety.rs         # Bash blacklist
│   ├── parser.rs         # Tool call parsing
│   └── grep_find.rs      # rg/find wrappers
├── runie-tui/src/
│   ├── ui.rs             # draw_snapshot
│   ├── popups/           # Panel/Form rendering
│   ├── theme.rs          # Color definitions
│   └── markdown.rs       # md → styled spans
├── runie-provider/src/
│   ├── openai.rs         # OpenAI-compatible providers
│   ├── anthropic.rs      # Anthropic
│   ├── model.rs          # Model catalog
│   └── config.rs         # Provider config
├── runie-term/src/
│   ├── main.rs           # Event loop, task dispatch
│   └── keymap.rs         # Key → Event mapping
├── runie-print/          # Print mode binary
├── runie-json/           # JSON mode binary
└── runie-server/         # RPC mode binary (crate exists, not wired)
```

## Reference implementations (in `~/Code/agents/`)

| Project     | Borrowed patterns                                            |
|-------------|--------------------------------------------------------------|
| pi          | Command registry, dialog DSL, session UX                     |
| crush       | Three-state collapse, lazy render cache                      |
| codex       | Event bus, `HistoryCell` trait, streaming tail buffer        |
| aider       | Repo map, edit previews, reflection loop, model capability flags |
| opencode    | Reasoning effort, multi-provider failover, context epochs    |
| goose       | MCP extension manager, swappable provider, message metadata  |
| gemini-cli  | Scheduler state machine, semantic theme tokens, tool display |
| thClaws     | `ViewEvent` abstraction, Braille spinner, append-only JSONL  |
| openharness | Tool registry, JSONL backend protocol, runtime bundle        |
| kimi-code   | Streaming UI controller, subagent host, semantic palette     |
| gptme       | Immutable log, hook lifecycle, context reduction pipeline    |
| etienne     | Session summaries, project dot-dir, SSE multiplex            |

## Configuration

`~/.config/runie/config.toml`:

```toml
provider = "anthropic"
model = "claude-3-5-sonnet"

[truncation]
max_lines = 2000
max_bytes = 51200

[models]
default = "claude-3-5-sonnet"
scoped = ["claude-3-5-sonnet", "gpt-4o", "claude-3-haiku"]

[prompts]
default = "default"
custom = "/path/to/prompts"

[telemetry]
enabled = false
```

Hot-reload: 2-second polling watcher emits `SwitchModel`/`SwitchTheme` on
change. Truncation is read once at startup (no hot-reload — would risk
in-flight tools getting a different policy mid-call).
