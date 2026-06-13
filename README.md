# Runie

**Agentic Harness with TUI.** You just register all the models you use, it makes sure you use them the most efficient way.

```bash
cargo build --release
./target/release/runie
```

## Overview

Runie is a terminal-based AI coding harness that provides an interactive TUI for multi-model agent execution. It combines streaming responses, tool execution, permission gating, and multi-agent orchestration in a unified interface.

## Architecture

Runie is split into focused crates. The `runie-term` binary is the TUI entry point; `runie-core` owns state, events, commands, and provider metadata; `runie-agent` runs the turn loop; and `runie-provider` implements the unified provider client.

```
┌─────────────────────────────────────────────────────────────┐
│                        runie-term                           │
│                   (TUI binary / CLI)                        │
└───────────────────────┬─────────────────────────────────────┘
                        │
    ┌───────────────────┼───────────────────┐
    │                   │                   │
┌───▼─────┐      ┌──────▼──────┐     ┌──────▼──────┐
│runie-   │      │  runie-     │     │   runie-    │
│agent    │◄────►│  provider   │     │   core      │
│(turns)  │      │(OpenAI API) │     │(state/types)│
└─────────┘      └─────────────┘     └─────────────┘
                        │
              ┌─────────▼─────────┐
              │    runie-tui      │
              │  (widgets/layout) │
              └───────────────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| `runie-core` | Shared state, events, commands, keybindings, provider registry |
| `runie-tui` | Terminal UI widgets, layout, and render helpers |
| `runie-term` | TUI binary entry point and terminal setup |
| `runie-agent` | Agent loop engine, event streaming, permission gating |
| `runie-provider` | Unified OpenAI-compatible provider client |
| `runie-print` | Plain-text printing utilities |
| `runie-json` | JSON output utilities |
| `runie-server` | Server/RPC mode components |

## Model Support

All providers use a single OpenAI-compatible API client. You switch provider/model at runtime with `/model <provider>/<model>`; an unknown provider returns an error instead of silently falling back.

| Provider | Key | Models |
|----------|-----|--------|
| Anthropic | `anthropic` | claude-sonnet-4-6, claude-opus-4-7, claude-haiku-4-5 |
| OpenAI | `openai` | gpt-4o, gpt-4o-mini, gpt-5, o3-mini, o4-mini |
| Google Gemini | `google` | gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash |
| DeepSeek | `deepseek` | deepseek-v4-flash, deepseek-v4-pro |
| OpenRouter | `openrouter` | anthropic/claude-sonnet-4.6, openai/gpt-4o, google/gemini-2.5-pro |
| Groq | `groq` | llama-3.3-70b-versatile, gemma2-9b-it, mixtral-8x7b-32768 |
| Mistral | `mistral` | mistral-large-latest, codestral-latest, devstral-latest |
| Fireworks | `fireworks` | accounts/fireworks/models/deepseek-v4-pro, accounts/fireworks/models/kimi-k2p6 |
| Together AI | `together` | meta-llama/Llama-3.3-70B-Instruct-Turbo, deepseek-ai/DeepSeek-V4-Pro |
| MiniMax | `minimax` | MiniMax-M3, MiniMax-M2.7 |
| Moonshot AI | `moonshotai` | kimi-k2.5, kimi-k2.6, kimi-k2-thinking |
| xAI | `xai` | grok-3, grok-4.3 |
| Ollama | `ollama` | llama3.1, qwen2.5-coder:7b, mistral |

## Features

### TUI Components

| Component | Description |
|-----------|-------------|
| **Feed** | Scrollable message list with assistant/user/tool messages, code blocks |
| **Input Bar** | Multi-line input with history (`↑`/`↓`), shift+enter for newlines |
| **Global Tags** | Token count, cost display; spinner + status during execution |
| **Top Bar** | Model indicator, session info, background job status |
| **Status Bar** | Agent state, thinking indicator, turn counter |
| **Command Palette** | Fuzzy-matched commands (`Ctrl+P`), usage tracking |
| **Permission Modal** | Tool execution approval with Allow/Deny/Skip/AllowAlways |
| **Model Picker** | Switch models mid-session |
| **Session Tree** | Browse/fork conversation branches |
| **Diff Viewer** | Side-by-side file diffs |

### Slash Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `/approve` | | Apply pending file edits |
| `/changelog` | | Show changelog |
| `/clone` | | Clone current session position |
| `/compact` | | Compact context |
| `/copy` | | Copy last response to clipboard |
| `/delete` | | Delete a saved session |
| `/diagnostics` | | Show resource loading diagnostics |
| `/export` | | Export session to JSON |
| `/fork` | | Fork session from a message |
| `/help` | `/h`, `/?` | Show available commands |
| `/history` | | Show recent history |
| `/import` | | Import session from JSON |
| `/load` | | Load a saved session |
| `/model` | `/m` | Switch model |
| `/name` | | Set session display name |
| `/new` | | Start new session |
| `/prompt` | | Switch prompt template |
| `/provider` | `/providers` | Manage providers |
| `/providers` | | Manage providers: add, disconnect, switch models |
| `/readonly` | `/ro` | Toggle read-only mode |
| `/reject` | | Cancel pending file edits |
| `/reload` | | Reload config, keybindings, themes |
| `/reset` | | Clear all state |
| `/resume` | | Resume most recent session |
| `/save` | | Save current session |
| `/scoped-models` | | Enable/disable models for cycling |
| `/session` | | Show session info |
| `/sessions` | | List saved sessions |
| `/share` | | Share session as GitHub gist |
| `/skill` | | Show skill details |
| `/skills` | | List loaded skills |
| `/spawn` | | Run a subagent turn (delegated task) |
| `/thinking` | | Set thinking level (off/low/medium/high) |
| `/theme` | | Switch theme or list available themes |
| `/tree` | | Open session tree dialog |
| `/trust` | | Trust current project |
| `/untrust` | | Untrust current project |
| `/quit` | `/q`, `/exit` | Quit application |

### Tool Execution

Tools are permission-gated with caching:

| Tool | Description |
|------|-------------|
| `bash` | Execute shell commands |
| `read_file` | Read file contents |
| `write_file` | Create/overwrite files |
| `edit_file` | In-place file edits |
| `search` | Search by name or content pattern |

Permission decisions:
- **Allow** — single execution
- **AllowAlways** — cached for session
- **Skip** — skip this tool call
- **Deny** — reject

### Multi-Agent Orchestration

```rust
// Spawn subagents for parallel tasks
let handle = orchestrator.spawn(task, &context).await?;

// Handoff context between agents
orchestrator.handoff(from, to, &context).await?;

// Collect results
let results = orchestrator.collect(handles).await?;
```

Features: task priorities (Low/Medium/High/Critical), max turns limits, tool allowlists, read-only mode.

### Configuration

Layered resolution (later layers override earlier):

1. Defaults
2. Global config (`~/.runie/config.toml` or `RUNIE_HOME/config.toml`)
3. Project config (`.runie/config.toml`)
4. Environment variables (`RUNIE_MODEL`, `RUNIE_PROVIDER`, `RUNIE_API_KEY`, etc.)
5. CLI arguments

```toml
# ~/.runie/config.toml
model = "gpt-4o"
provider = "openai"
max_turns = 10
enable_thinking = true
```

## Quick Start

```bash
# Build
cargo build --release

# Interactive TUI
./target/release/runie

# With mock provider (no API key)
RUNIE_MOCK=1 ./target/release/runie

# CLI one-shot mode
./target/release/runie run "Explain this code"

# Custom config directory
./target/release/runie --dev-folder=./tmp_config

# Resume session
./target/release/runie --session <session-id>
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+B` | CursorWordLeft |
| `Alt+Enter` | FollowUp |
| `Alt+F` | CursorWordRight |
| `Alt+Up` | Dequeue |
| `Backspace` | Backspace |
| `Ctrl+A` | CursorStart |
| `Ctrl+B` | CursorLeft |
| `Ctrl+C` | Quit |
| `Ctrl+D` | KillChar |
| `Ctrl+E` | ToggleExpand |
| `Ctrl+F` | CursorRight |
| `Ctrl+G` | OpenExternalEditor |
| `Ctrl+J` | Newline |
| `Ctrl+K` | DeleteToEnd |
| `Ctrl+M` | CycleModelNext |
| `Ctrl+P` | ToggleCommandPalette |
| `Ctrl+S` | Abort |
| `Ctrl+Shift+M` | CycleModelPrev |
| `Ctrl+Shift+P` | ToggleCommandPalette |
| `Ctrl+U` | DeleteToStart |
| `Ctrl+V` | PasteImage |
| `Ctrl+W` | DeleteWord |
| `Ctrl+Y` | Redo |
| `Ctrl+Z` | Suspend |
| `Delete` | KillChar |
| `Down` | HistoryNext |
| `End` | CursorEnd |
| `Enter` | Submit |
| `Escape` | DialogBack |
| `Home` | CursorStart |
| `Left` | CursorLeft |
| `PageDown` | PageDown |
| `PageUp` | PageUp |
| `Right` | CursorRight |
| `Shift+Enter` | Newline |
| `Shift+Tab` | CycleThinkingLevel |
| `Tab` | Input:\t |
| `Up` | HistoryPrev |

## Development

```bash
# Run with dev config and mock provider
./dev.sh

# Run tests
cargo test --workspace

# Clippy
cargo clippy --workspace

# Format
cargo fmt
```

Build enforces:
- Max 500 lines per file
- Max 40 lines per function
- Max complexity 10 per function

Set `RUNIE_SKIP_BUILD_CHECKS=1` to bypass.

## Dependencies

Key dependencies (from `Cargo.lock`):
- `ratatui` — TUI rendering
- `tokio` — async runtime
- `reqwest` — HTTP client
- `serde` — serialization
- `chrono` — timestamps
- `uuid` — session IDs

## License

MIT
