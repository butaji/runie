# Runie Specification

Terminal coding agent harness in Rust, inspired by [pi](https://pi.dev).

> **Snapshot: 2026-06-11.** For historical design rationale, see `docs/archive/`
> and the ADRs. For task history, see `tasks/`.

## Architecture

### Runtime

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EventBus<CoreEvent>                            │
│  (tokio::sync::broadcast + bounded replay buffer)                        │
└─────────────────────────────────────────────────────────────────────────┘
        ▲            ▲            ▲            ▲            ▲
        │            │            │            │            │
┌───────┴────┐ ┌─────┴──────┐ ┌──┴─────────┐ ┌┴───────────┐ ┌───────────┐
│ InputActor │ │ AgentActor │ │ ConfigActor│ │SessionActor│ │  UiActor  │
│ (crossterm)│ │(LLM+tools) │ │(TOML watch)│ │(JSONL store)│ │(AppState  │
└────────────┘ └────────────┘ └────────────┘ └─────────────┘ │ projection)│
                                                             └─────┬─────┘
                                                                   │
                                                             Snapshot
                                                                   │
                                                             ┌─────┴─────┐
                                                             │ RenderActor│
                                                             │  (ratatui) │
                                                             └───────────┘
```

Actors are tokio tasks that publish and subscribe to a typed `EventBus`.
`SessionActor` persists durable events to append-only JSONL files.
`UiActor` projects events into `AppState` and sends snapshots to the render
actor via a `watch` channel. State is owned by the actors/projection actors,
not by a central loop.

### Crates

| Crate            | Role                                                        |
|------------------|-------------------------------------------------------------|
| `runie-core`     | Events, AppState, sessions, config, commands                |
| `runie-agent`    | Tool implementations, agent turn, subagent, truncation      |
| `runie-provider` | LLM providers, model catalog                                |
| `runie-tui`      | Ratatui rendering, panels/forms, theme                      |
| `runie-term`     | CLI entry, subagent dispatcher, smoke harness               |
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
events (UI-only). The previous single `Event` enum is being decomposed into
focused sub-enums (`tasks/event-subenums.md`).

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
- Diagnostics, reload, suspend, share, external editor
- TUI features: streaming, markdown, syntax highlight, diff, ANSI, scrollbar,
  input history, undo/redo, multi-line, @-file refs, path completion
- Image paste (Ctrl+V)
- Subagents (`/spawn <prompt>`)
- Modes: interactive TUI, print, JSON, RPC/server

### Roadmap (R3)

Planned architecture and UX improvements based on research in `~/Code/agents`:

- **Event-based actor runtime** — tokio-task actors + `EventBus` + JSONL session
  persistence (`tasks/actor-runtime-decision.md`,
  `tasks/event-bus-jsonl-persistence.md`)
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

### Test coverage

- ~1,060 automated tests across the workspace, all passing
- 4-layer TDD per `AGENTS.md`: state/logic, event handling, rendering, smoke
- Lint: zero build violations
- Pre-existing failures: 4 scrollbar/AT-lookup render tests (unrelated to
  recent work; tracked in tasks/)

### Out of scope (by design)

- **Plugins/extensions** — adds complexity without daily-use value
- **OAuth login flow** — API keys in config.toml suffice
- **Subagent parallel orchestration / DAG** — single linear subagent
- **Session tree / branching UI** — `/fork` exists, but no visual tree
- **Custom syntax-highlighting languages** — limited built-in tokenizers; full grammar support via crate audit (`tasks/crate-replacement-audit.md`)
- **Web UI / VS Code extension** — terminal-only

## Code organization

```
crates/
├── runie-core/src/
│   ├── actor.rs          # Minimal Actor trait
│   ├── bus.rs            # EventBus<CoreEvent> with replay
│   ├── event.rs          # CoreEvent + durable/transient split
│   ├── llm_event.rs      # Provider-agnostic LLM event enum
│   ├── model.rs          # AppState, ChatMessage
│   ├── state.rs          # Sub-state structs (config, input, ...)
│   ├── tool.rs           # Tool trait + ToolRegistry
│   ├── mcp.rs            # MCP client + config types
│   ├── permissions.rs    # Permission rulesets + ApprovalSink
│   ├── context_compactor.rs # Token-threshold compaction
│   ├── streaming_buffer.rs  # Stable region + mutable tail
│   ├── session_store.rs  # JSONL persistence + session index
│   ├── config_reload.rs  # TruncationSection + config watcher
│   ├── session.rs        # Session types (legacy; migrate to session_store)
│   ├── snapshot.rs       # View projection
│   ├── skills/           # SKILL.md loading
│   ├── prompts/          # Prompt templates
│   ├── commands/         # CommandRegistry + handlers/
│   ├── dialog/           # Panel/Form DSL + PanelStack
│   ├── update/           # Event dispatch (mod, input, agent, ...)
│   └── telemetry.rs      # Opt-in usage stats
├── runie-agent/src/
│   ├── tools/            # One module per built-in Tool impl
│   ├── tools.rs          # Built-in ToolRegistry assembly
│   ├── turn.rs           # Agent turn loop consuming LLMEvent
│   ├── subagent.rs       # Nested turn for /spawn
│   ├── truncate.rs       # TruncationConfig (TOML) + policies
│   ├── accumulator.rs    # Bounded buffer for streaming
│   ├── mutation_queue.rs # Serialized file edits
│   ├── safety.rs         # Bash blacklist
│   ├── parser.rs         # Tool call parsing (legacy; retire after LLMEvent)
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
│   ├── main.rs           # Event loop, subagent dispatch
│   └── keymap.rs         # Key → Event mapping
├── runie-print/          # Print mode binary
├── runie-json/           # JSON mode binary
└── runie-server/         # RPC mode binary
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
| autogen     | Workbench abstraction, message/event taxonomy                |
| crewai      | Typed event bus, tool lifecycle events, checkpoint runtime   |
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
