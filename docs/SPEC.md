# Runie Specification

Terminal coding agent harness in Rust, inspired by [pi](https://pi.dev).

## Architecture

See [ADR documentation](./adr/) for architectural decisions.

### Actor Hierarchy

```
Event Bus
    │
    ├── Orchestrator (spawns all actors, routes messages)
    │   │
    │   ├── AgentLoop ────→ ToolActors
    │   ├── QueueAgent ────→ message queue, batching
    │   ├── SessionManager ────→ session persistence
    │   ├── ConfigAgent ────→ TOML loading, hot reload
    │   ├── TelemetryAgent ────→ token/cost tracking
    │   ├── SafetyAgent ────→ bash validation
    │   ├── ClipboardAgent ────→ image paste
    │   ├── FileLookupActor ────→ @-file resolution
    │   ├── CommandAgent ────→ slash commands, key shortcuts
    │   └── Skills (interceptors on bus)
    │
    └── UIRoot (routes UI events to children)
        │
        ├── InputAgent ────→ input, cursor, history
        ├── ScrollAgent ────→ scroll, viewport
        ├── ChatAgent ────→ elements, streaming
        └── PopupAgent ────→ hints, @-suggestions

View (runie-tui) ────→ pure function: UIAgent state → Frame
```

### Crate Responsibilities

| Crate | Responsibility |
|-------|---------------|
| `runie-core` | Domain events, AppState, session persistence, Provider trait |
| `runie-tui` | UI actors, ratatui rendering, Element enum |
| `runie-agent` | Tool implementations, truncation |
| `runie-provider` | OpenAI, Anthropic, model registry |
| `runie-term` | CLI entry point, crossterm input |

### Event Model

Events are tagged as **domain** (persisted) or **ephemeral** (not persisted).

Domain events: `Submit`, `SpawnAgent`, `AgentThinking`, `AgentResponse`, `ToolStart`, `ToolEnd`, `Done`, `SwitchModel`, `ToolRegistered`

Ephemeral events: `ScrollUp`, `CursorLeft`, `Paste`, `ToggleExpand`, etc.

---

## Feature Milestones

---

## MVP

### Core Architecture
- [x] Actor-based event-driven architecture
- [x] Shared event bus with typed channels
- [x] Orchestrator spawning all actors
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
- [x] Hot reload on config change

---

## R1 (Code Quality + User Value)

### Core Refactor
- [x] **Split update.rs** — Divided into `update/{mod,input,agent,slash,queue}.rs`
- [ ] **Compose AppState** — Split 27-field god object into `InputState`, `ChatHistory`, `AgentState`, `UiState`
- [x] **Fix clippy warnings** — Zero errors in production code
- [x] **Cache optimizations** — O(1) `append_response` via `last_assistant_index`
- [ ] **Remove dead code** — `VisibleRegion` still referenced by autoscroll tests

### Agent Crate Cleanup
- [ ] **Module split** — Divide `runie-agent/src/lib.rs` into `turn.rs`, `tools.rs`, `truncate.rs`, `safety.rs`, `parser.rs`

### TUI Render Cleanup
- [ ] **Split render tests** — Divide `tests/render.rs` (>500 lines) into focused modules

### Actors (keep existing infrastructure, extend where needed)
- [ ] **ToolActors** — Spawn per tool invocation, self-describe via ToolRegistered
- [ ] **QueueAgent** — Manages message queue with configurable batching
- [ ] **SessionManager** — Handles session save/load/list/delete
- [ ] **ConfigAgent** — Loads TOML config, watches for changes

### TUI Improvements
- [ ] **Streaming: event per chunk** — Each LLM chunk emitted as individual event
- [ ] **Ctrl+Shift+E** — Collapse/expand feed elements
- [ ] **!command** — Bash prefix (run bash, don't send to agent)

### Configuration
- [ ] **Configurable keybindings** — Loaded from `keybindings.json` via ConfigAgent

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
│       ├── session.rs     # Session persistence
│       ├── snapshot.rs    # View state snapshot
│       └── provider.rs    # Provider trait
│
├── runie-agent/           # Tool implementations
│   └── src/
│       ├── tools.rs       # Tool enum, execution
│       ├── truncate.rs    # Output truncation
│       ├── safety.rs      # Bash validation
│       └── parser.rs      # Tool call parsing
│
├── runie-tui/             # UI actors + rendering
│   └── src/
│       ├── ui.rs          # Ratatui rendering
│       ├── markdown.rs     # Markdown parsing
│       ├── theme.rs       # Color/theme definitions
│       └── ui/
│           ├── input_agent.rs
│           ├── scroll_agent.rs
│           ├── chat_agent.rs
│           └── popup_agent.rs
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
        ├── main.rs        # Binary entry
        ├── bus.rs         # EventBus implementation
        ├── orchestrator.rs # Actor orchestration
        ├── queue_agent.rs  # Queue management
        └── commands.rs     # Slash command parsing
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

- Architecture follows actor model with shared event bus
- Event log is source of truth for session persistence
- UI is pure MVU projection from event stream
- Skills are lightweight interceptors, not full actors
- Non-interactive modes (print, json) bypass actor system entirely
