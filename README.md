# Runie

An AI coding assistant with a terminal UI, designed for speed and local-first execution.

## Crates

| Crate | Description |
|-------|-------------|
| `runie-core` | Config, providers, permissions, actors, event bus |
| `runie-provider` | Model provider clients (OpenAI, Anthropic, MiniMax, etc.) |
| `runie-agent` | Agent loop: tool use, streaming turns, history |
| `runie-cli` | `runie` binary: print/json/server/mcp/login modes |
| `runie-tui` | `runie-tui` binary: TUI with ratatui |
| `runie-testing` | Test harness (tmux driver, replay helpers) |
| `runie-patterns` | Pattern matching utilities |
| `runie-acp` | ACP protocol implementation |

## Quick Start

```bash
cargo build --release
cargo test
```

## Architecture

Runie uses an events-based, single-source-of-truth actors architecture:
- Each state slice is owned by exactly one actor
- The only change mechanism is events published by the owning actor
- Every spawned task has an owner

See [AGENTS.md](./AGENTS.md) for detailed guidelines.

## License

MIT OR Apache-2.0
