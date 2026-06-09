# Runie Specification

Terminal coding agent harness in Rust, inspired by [pi](https://pi.dev).

## Architecture

See [ADR documentation](./adr/) for architectural decisions.

### Runtime Architecture

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

Three tokio tasks + one event loop. State is owned by the event loop, mutated
per event. Snapshots are sent to the render task via `mpsc::channel`.

Historical note: An actor system (EventBus, Orchestrator, typed channels) was
built during MVP but is not used by the runtime. It has been identified as dead
code. See `docs/SHIP_REVIEW_2.md`.

### Crate Responsibilities

| Crate | Responsibility |
|-------|---------------|
| `runie-core` | Domain events, AppState, session persistence, Provider trait |
| `runie-tui` | Ratatui rendering, Element enum, theme |
| `runie-agent` | Tool implementations, truncation |
| `runie-provider` | OpenAI, Anthropic, model registry |
| `runie-term` | CLI entry point, crossterm input |

### Event Model

All UI and agent events are unified into a single `Event` enum in
`runie-core/src/event.rs`. Events are handled by `AppState::update()` which
mutates state directly (logically pure, mechanically mutable for zero-copy).

There is no separate domain/ephemeral split at the type level — all events flow
through the same channel and reducer.

---

## Feature Milestones

---

## MVP

### Core Architecture
- [x] Event-driven architecture (tokio async, mpsc channels)
- [x] Shared event bus with typed channels
- [x] Async runtime with non-blocking render
- [x] Async/await runtime (tokio)

### Providers & Models
- [x] Provider catalog (OpenAI, Anthropic, OpenRouter, etc.)
- [x] Model registry with runtime switch
- [x] API key authentication via environment variables
- [x] Token and cost tracking

### Tools
- [x] `bash` - Execute shell commands with safety guards
- [x] `read` - Read file contents with line limits
- [x] `write` - Write complete file contents
- [x] `edit` - Search/replace with unique match validation
- [x] `ls` - List directory contents
- [x] `grep` - Search with regex/literal/glob support
- [x] `find` - Glob-based file finding with .gitignore support
- [x] Output truncation (lines + bytes limits)

### TUI Rendering
- [x] Streaming response merge by request ID
- [x] Sort by last update (elements float to bottom)
- [x] Token count in footer
- [x] Queue count in footer
- [x] Thinking text display
- [x] Thinking collapse to single-line summary
- [x] Tool output collapse to single-line summary
- [x] Word wrapping
- [x] Markdown rendering
- [x] Diff rendering for edits
- [x] Syntax highlighting for code blocks
- [x] ANSI color support
- [x] Scrollbar

### Sessions
- [x] Save/load sessions to JSONL files
- [x] List/delete sessions
- [x] Session persistence across restarts

### Input & Commands
- [x] Slash commands: `/model`, `/save`, `/load`, `/sessions`, `/delete`, `/reset`, `/help`, `/compact`
- [x] Message queue: steering (Enter) + follow-up (Alt+Enter) + abort (Esc)
- [x] @-file reference detection
- [x] Multi-line input support
- [x] Input history

### Safety
- [x] Bash blacklist (blocks `rm -rf /`, `dd`, `mkfs`, fork bombs)
- [x] Output size limits

### Configuration
- [x] TOML configuration (`~/.runie/config.toml`)
- [ ] Hot reload on config change (deferred to R1)

---

## R1 (User Value)

### Already Done
- [x] **Split update.rs** — Divided into `update/{mod,input,agent,slash,queue}.rs`
- [x] **Fix clippy warnings** — Zero errors in production code
- [x] **Cache optimizations** — O(1) `append_response` via `last_assistant_index`
- [x] **Ctrl+Shift+E** — Collapse/expand feed elements
- [x] **!command** — Bash prefix (run bash, don't send to agent)

### Remaining (Prioritized)
- [ ] **Configurable keybindings** — Load from `keybindings.json`, dispatch via map
- [ ] **Streaming: event per chunk** — Each LLM chunk emitted as individual event
- [ ] **Hot reload** — File watcher for config changes
- [ ] **Input history persistence** — Save history across sessions

### Deferred (Not Blocking)
- [ ] **Compose AppState** — Nice-to-have; 27 fields work fine
- [ ] **Remove dead code** — `VisibleRegion` cleanup when tests are rewritten

---

## R2

### Providers & Models
- [ ] **Model cycling** — Ctrl+P opens commands panel
- [ ] **Scoped model filtering** — Enable/disable models for cycling
- [ ] **Model selector UI** — Interactive picker for model selection
- [ ] **Provider attribution** — Show which provider served the response
- [ ] **OAuth authentication** — `/login`, `/logout` per provider
- [ ] **Dynamic provider config** — Resolve config from env, files, CLI flags

### Sessions
- [ ] **Session branching** — `/fork`, `/clone`, `/tree` for fork from any message
- [ ] **Session naming** — `/name` sets display name
- [ ] **Session info/stats** — `/session` shows metadata
- [ ] **Export to HTML** — `/export` creates shareable HTML
- [ ] **Import from JSONL** — `/import` resumes a session
- [ ] **Session tree navigation** — Visual tree with fold/unfold, labels, filters
- [ ] **Session filters** — no-tools, user-only, labeled-only, all

### TUI Rendering
- [ ] **Thinking levels** — Shift+Tab cycles low/medium/high reasoning
- [ ] **Path completion** — Tab completion for paths in input
- [ ] **Multi-line input** — Shift+Enter for newlines, Ctrl+J for newlines
- [ ] **Image paste** — Ctrl+V paste from clipboard
- [ ] **Read-only tool mode** — Restrict to read/grep/find/ls only

### Input & Commands
- [ ] **Additional slash commands**:
  - `/export`, `/import`, `/share`, `/copy`
  - `/name`, `/session`
  - `/fork`, `/clone`, `/tree`
  - `/trust`, `/login`, `/logout`
  - `/new`, `/resume`, `/reload`, `/changelog`, `/hotkeys`
- [ ] **Dequeue** — Alt+Up restores queued messages
- [ ] **Skills system** — Load SKILL.md files from user/project directories
- [ ] **Custom prompt templates** — User-defined system prompt overrides
- [ ] **Context files** — Load AGENTS.md, CLAUDE.md from project

### Keybindings
- [ ] **External editor** — Ctrl+G opens $EDITOR
- [ ] **Paste image** — Ctrl+V (Alt+V on Windows)
- [ ] **Suspend to background** — Ctrl+Z

### Configuration
- [ ] **Settings UI/menu** — `/settings` interactive menu
- [ ] **Theme system** — Customizable terminal themes with hot reload

### Modes
- [ ] **Print mode** — Non-interactive CLI output (`runie-print`)
- [ ] **JSON mode** — Structured JSON output for scripting (`runie-json`)
- [ ] **RPC/server mode** — Expose agent capabilities over RPC

### Safety
- [ ] **Trust system** — `/trust` per-project decision
- [ ] **Output guard** — Accumulator limits tool output size
- [ ] **File mutation queue** — Serialize file edits to avoid conflicts
- [ ] **Edit diff preview** — Show diff before applying edit

---

## R3

### Extensions
- [ ] **Extension system** — TypeScript/npm-style plugin architecture
- [ ] **Custom tools via extensions** — Register tools callable by LLM
- [ ] **Event interception** — Block/modify tool calls, inject context
- [ ] **Custom commands** — Register commands like `/mycommand`
- [ ] **Custom UI components** — Full TUI components with keyboard input

### Sessions
- [ ] **Export to GitHub gist** — `/share` uploads as private gist with HTML link
- [ ] **Branching with summaries** — AI-generated branch summaries on navigation

### Tools
- [ ] **Path utilities** — Full cwd resolution
- [ ] **Structured JSON tools** — Enhanced JSON parsing
- [ ] **MCP server integration** — Connect to MCP servers as tools
- [ ] **MCP resources** — Read/write MCP resources

### SDK
- [ ] **Programmatic embedding** — Use runie as library in Rust programs
- [ ] **Subprocess integration** — RPC mode for language-agnostic clients
- [ ] **ACP (Agent Client Protocol)** — Drive session from editors (Zed, JetBrains)

### Telemetry
- [ ] **Opt-in telemetry** — Usage statistics collection
- [ ] **Diagnostics** — Resource loading diagnostics
- [ ] **Update checks** — Check for newer versions

### Pi Packages
- [ ] **Pi packages** — Bundle and share extensions, skills, prompts, themes
- [ ] **Package registry** — Install from npm/git

### UI/UX
- [ ] **Custom scrollback** — Configurable scrollback buffer
- [ ] **Notifications** — System notifications for long operations
- [ ] **Sound effects** — Audio feedback for events

### Developer Experience
- [ ] **Debugger integration** — Step through agent reasoning
- [ ] **Session replay** — Replay and analyze past sessions
- [ ] **Performance profiling** — Profile token usage and latency

### Provider Features
- [ ] **Multi-provider failover** — Fallback to backup provider on failure
- [ ] **Cost optimization** — Auto-select cheaper models for simple tasks

### Advanced Features
- [ ] **Split turn handling** — Handle single turns exceeding token budget
- [ ] **Custom summarization** — Extension hooks for custom compaction
- [ ] **LSP integration** — Language server protocol for context

---

## File Structure

```
crates/
├── runie-core/           # Domain logic, events, AppState
│   └── src/
│       ├── event.rs      # Unified Event type
│       ├── model.rs      # AppState, ChatMessage
│       ├── update/        # State transitions (mod, input, agent, slash, queue)
│       ├── ui/            # Element enum, transforms, lazy cache
│       ├── session.rs     # Session persistence (simple JSON)
│       ├── snapshot.rs    # View state snapshot
│       ├── provider.rs    # Provider trait
│       └── labels.rs      # Static text constants
│
├── runie-agent/           # Tool implementations
│   └── src/
│       ├── tools.rs       # Tool enum, execution
│       ├── turn.rs        # Agent turn loop
│       ├── truncate.rs    # Output truncation
│       ├── safety.rs      # Bash validation
│       ├── parser.rs      # Tool call parsing
│       ├── diff.rs        # Edit diff logic
│       └── grep_find.rs   # Grep/find utilities
│
├── runie-tui/             # Ratatui rendering
│   └── src/
│       ├── ui.rs          # Widget rendering, draw_snapshot
│       ├── markdown.rs     # Markdown → styled spans
│       ├── syntax.rs       # Syntax highlighting
│       ├── theme.rs       # Color definitions
│       └── diff.rs        # Diff rendering
│
├── runie-provider/         # LLM provider implementations
│   └── src/
│       ├── openai.rs      # OpenAI provider
│       ├── anthropic.rs    # Anthropic provider
│       ├── model.rs       # Model registry
│       └── config.rs      # Provider config
│
└── runie-term/            # CLI entry point
    └── src/
        ├── main.rs        # Event loop, key mapping, render task
        └── tests/         # Integration/render tests
```

---

## Reference Implementations

| Project | Strengths |
|---------|-----------|
| [pi](https://pi.dev) | Extension system, session tree, theme system, OAuth |
| [crush](https://github.com/bootandy/crush) | Thinking/tool collapse, three-state toggle, MCP |
| [codex](https://github.com/openai/codex) | Token-aware truncation, middle truncation |
| [aider](https://aider.chat) | Repo map, git integration, voice-to-code |
| [goose](https://github.com/aaif-goose/goose) | Session management, conversation history |
| [opencode](https://github.com/opencode-ai/opencode) | Reasoning effort, model variants |
| [kimi-code](https://github.com/MoonshotAI/kimi-code) | Goals, MCP configuration, ACP |
| [langgraph](https://github.com/langchain-ai/langgraph) | Durable execution, human-in-the-loop |

---

## Notes

- Runtime uses three tokio tasks + single-threaded event loop
- State is owned by the event loop; snapshots are MVU projections
- Sessions use simple JSON files (~/.runie/sessions/)
- An unused actor system exists in the codebase (event_bus, orchestrator, actors/)
  but is not wired into the runtime. See `docs/SHIP_REVIEW_2.md`.
- Non-interactive modes (print, json) are future work
