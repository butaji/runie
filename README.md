# Runie

Terminal-native harness for LLM-powered coding agents. One session, every model, permission-gated tools.

## Quick start

```bash
cargo build --release

# Interactive TUI
./target/release/runie-tui

# Mock provider (no API key)
RUNIE_MOCK=1 ./target/release/runie-tui

# One-shot CLI
./target/release/runie print "find unused imports"
```

Add provider keys in `~/.runie/config.toml`. See [docs/Configuration.md](docs/Configuration.md).

## What it does

|  |  |
|---|---|
| Multi-model routing | Switch provider/model mid-session with `/model anthropic/claude-sonnet-4-6` |
| Permission-gated tools | Read, list, write, edit, bash, grep, find, fetch docs — all ask first |
| Forkable sessions | Save, load, fork, branch. Local JSONL history |
| Terminal-native | Keyboard-driven TUI with `@` file references and slash commands |
| Scoped model cycling | Rotate a configured shortlist with one hotkey |

## Modes

| Mode | Command |
|---|---|
| TUI | `./target/release/runie-tui` |
| Print | `./target/release/runie print "..."` |
| Inspect | `./target/release/runie inspect` |
| Login | `./target/release/runie login` |
| JSON | `./target/release/runie json` |
| Server | `./target/release/runie server` |
| MCP | `./target/release/runie mcp list\|add\|remove` |

## Providers

Anthropic · OpenAI · Google Gemini · DeepSeek · OpenRouter · Groq · Fireworks · Together · MiniMax · Moonshot AI · xAI · Mistral · Ollama

## Development

```bash
just tui --mock                       # run TUI with mock provider
just tui --mock --mock-model list_dir # deterministic mock fixture
just test                             # workspace tests
just lint                             # clippy
just fmt                              # format check
just --list                           # all recipes
```

See `AGENTS.md` for conventions and `docs/Architecture.md` for the runtime model.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT) at your option.
