# Runie

**Agentic Harness with TUI.** You just register all the models you use, it makes sure you use them the most efficient way.

```bash
cargo build --release
./target/release/runie
```

## Overview

Runie is a terminal-based AI coding harness that provides an interactive TUI for multi-model agent execution. It combines streaming responses, tool execution, permission gating, and multi-agent orchestration in a unified interface.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      runie-cli                          │
│                   (CLI entry point)                     │
└─────────────────────┬───────────────────────────────────┘
                      │
         ┌───────────┴───────────┐
         │                       │
    ┌────▼────┐            ┌──────▼─────┐
    │runie-tui│            │  harness   │
    │   TUI   │            │  (tests)   │
    └────┬────┘            └──────┬─────┘
         │                       │
    ┌────▼───────────────────────▼─────┐
    │         runie-agent              │
    │   Agent loop + streaming + tools │
    └────┬──────────────────────────────┘
         │
    ┌────▼────┐     ┌────────────┐     ┌───────────┐
    │runie-ai │────▶│runie-tools │────▶│runie-core │
    │Provider │     │  Tools     │     │  Types    │
    └─────────┘     └────────────┘     └───────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| `runie-core` | Shared types: Message, Tool, Session, Context, SlashCommand |
| `runie-ai` | Model providers with unified API, streaming, token usage |
| `runie-agent` | Agent loop engine, event streaming, permission gating |
| `runie-tools` | Tool implementations: bash, read/write/edit files, search |
| `runie-tui` | Terminal UI: feed, input, global tags, top bar, command palette |
| `runie-cli` | CLI binary (`runie`) and `cargo-pantry` |
| `harness` | Eval/grading harness for task fixtures (Rust graders) |

## Model Support

Runie supports multiple providers through a unified provider interface:

| Provider | Models |
|----------|--------|
| **OpenAI** | gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4 |
| **Anthropic** | claude-3-5-sonnet, claude-3-opus, claude-3-sonnet, claude-3-haiku |
| **Google** | gemini-1.5-pro, gemini-1.5-flash, gemini-1.5-flash-8b, gemini-2.0-flash |
| **MiniMax** | MiniMax API compatible |
| **Rig** | OpenRouter-compatible endpoints |

## Features

### TUI Components

| Component | Description |
|-----------|-------------|
| **Feed** | Scrollable message list with assistant/user/tool messages, code blocks |
| **Input Bar** | Multi-line input with history (`↑`/`↓`), shift+enter for newlines |
| **Global Tags** | Token count, cost display; spinner + status during execution |
| **Top Bar** | Model indicator, session info, background job status |
| **Status Bar** | Agent state, thinking indicator, turn counter |
| **Command Palette** | Fuzzy-matched commands (`Ctrl+K`), usage tracking |
| **Permission Modal** | Tool execution approval with Allow/Deny/Skip/AllowAlways |
| **Model Picker** | Switch models mid-session |
| **Session Tree** | Browse/fork conversation branches |
| **Diff Viewer** | Side-by-side file diffs |

### Slash Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `/new` | `/n` | Start new session |
| `/clear` | `/c` | Clear conversation |
| `/model <name>` | `/m` | Switch model |
| `/tree` | `/t` | Open session tree |
| `/fork` | `/f` | Fork at current position |
| `/copy` | | Copy last response |
| `/cost` | | Show cost statistics |
| `/quit` | `/q`, `/exit` | Exit |
| `/help` | `/h` | Show help |

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
./target/release/runie --mock

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
| `Enter` | Submit message |
| `Shift+Enter` | New line in input |
| `Ctrl+C` | Exit |
| `Ctrl+O` | Copy last response |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+K` / `Ctrl+P` | Command palette |
| `Ctrl+N` | New session (via palette) |
| `Ctrl+L` | Clear chat (via palette) |

## Development

```bash
# Run with dev config
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
- `genai` / `rig-core` — AI provider integrations

## License

MIT
