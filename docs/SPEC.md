# Runie Specification

Terminal coding agent harness in Rust, inspired by [pi](https://pi.dev).

> **Snapshot: 2026-06-11.** For historical design rationale, see `docs/archive/`
> and the ADRs. For task history, see `tasks/`.

## Architecture

### Runtime

```
┌─────────────────┐     CoreEvent      ┌─────────────────┐
│  input_reader   │ ──────────────────>│                 │
│  (crossterm)    │                    │   event_loop    │
└─────────────────┘                    │   (owns state)  │
                                       │                 │
┌─────────────────┐     CoreEvent      │  ┌───────────┐  │
│   agent_loop    │ ──────────────────>│  │ AppState  │  │
│ (run_agent_turn)│                    │  └─────┬─────┘  │
└─────────────────┘                    │        │        │
                                       │   snapshot()    │
┌─────────────────┐     Snapshot       │        │        │
│  render_task    │ <──────────────────│  ┌─────┴─────┐  │
│   (ratatui)     │                    │  │ render_tx │  │
└─────────────────┘                    │  └───────────┘  │
                                       └─────────────────┘
```

Three tokio tasks + one event loop. State is owned by the event loop,
mutated per event. Snapshots are sent to the render task via `mpsc::channel`.

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

All events flow through a single `Event` enum in `runie-core/src/event.rs`.
`AppState::update()` is the single reducer. Synchronous; no separate domain
bus. Agent work runs in a separate tokio task and pushes events back through
`mpsc::channel`.

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
- **Custom syntax-highlighting languages** — uses syntect defaults
- **Web UI / VS Code extension** — terminal-only

## Code organization

```
crates/
├── runie-core/src/
│   ├── event.rs          # All event types
│   ├── model.rs          # AppState, ChatMessage
│   ├── state.rs          # Sub-state structs (config, input, ...)
│   ├── config_reload.rs  # TruncationSection + config watcher
│   ├── session.rs        # Session persistence
│   ├── snapshot.rs       # View projection
│   ├── skills/           # SKILL.md loading
│   ├── prompts/          # Prompt templates
│   ├── commands/         # CommandRegistry + handlers/
│   ├── dialog/           # Panel/Form DSL + PanelStack
│   ├── update/           # Event dispatch (mod, input, agent, ...)
│   └── telemetry.rs      # Opt-in usage stats
├── runie-agent/src/
│   ├── tools.rs          # Tool enum, execution
│   ├── turn.rs           # Agent turn loop
│   ├── subagent.rs       # Nested turn for /spawn
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
│   ├── main.rs           # Event loop, subagent dispatch
│   └── keymap.rs         # Key → Event mapping
├── runie-print/          # Print mode binary
├── runie-json/           # JSON mode binary
└── runie-server/         # RPC mode binary
```

## Reference implementations (in `~/Code/agents/`)

| Project   | Borrowed patterns                              |
|-----------|------------------------------------------------|
| pi        | Command registry, dialog DSL, session UX       |
| crush     | Three-state collapse, lazy render cache        |
| codex     | Token-aware truncation, structured JSON mode   |
| aider     | Repo map, edit previews                        |
| opencode  | Reasoning effort, multi-provider failover      |

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
