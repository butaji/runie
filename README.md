# Runie

> **The AI coding agent that lives in your terminal.**
>
> One keyboard-driven interface. Every model you use. Local files, local tools, and your full session history—without leaving the shell.

```bash
cargo build --release
./target/release/runie
```

---

## Why Runie exists

Most coding agents want you to leave your terminal: open a browser tab, paste context, watch a progress spinner, then copy code back by hand. Every switch costs focus. Every closed tab costs context.

Runie does the opposite. It runs where you already work: inside your terminal, next to your editor, with direct access to the codebase, shell, and git history. You stay in flow. The agent stays in context.

## What Runie is

Runie is a terminal-native harness for LLM-powered coding agents. It is not a chat website. It is not tied to one provider. It is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

In practice, Runie gives you four things in one window:

1. **A fast chat interface** with streaming responses, markdown, code blocks, and diffs.
2. **A multi-model router** so you can pick the right provider and model for each task, mid-session.
3. **A permission-gated tool runner** that lets the agent use `read_file`, `edit_file`, `write_file`, `bash`, `grep`, `find`, and more—only when you allow it.
4. **Persistent sessions** that save context, let you fork branches, and resume later.

## What you can do with it

- **Ask about your code.** Paste a file with `@path/to/file`, ask a question, get an explanation rooted in the actual source.
- **Refactor across files.** Describe the change; Runie proposes edits, shows a diff, and applies them when you say `/approve`.
- **Run one-off tasks.** `runie run "find all unused imports"` gives you a headless answer without opening the TUI.
- **Switch models on the fly.** `/model anthropic/claude-sonnet-4` for reasoning, `/model openai/gpt-4o` for speed. Same session, different brain.
- **Save and resume conversations.** `/save debug-session`, come back tomorrow with `/load debug-session`.

## Quick start

```bash
# Build
cargo build --release

# Start the interactive TUI
./target/release/runie

# Try it without an API key
RUNIE_MOCK=1 ./target/release/runie

# One-shot CLI mode
./target/release/runie run "Explain this code" < src/main.rs
```

On first launch, Runie opens a login flow. Add your provider API key, pick a default model, and start typing. Your config lives in `~/.runie/config.toml` and is hot-reloadable.

## A five-minute tour

```toml
# ~/.runie/config.toml
provider = "openai"
model = "gpt-4o"

[models]
scoped = ["gpt-4o", "claude-sonnet-4", "deepseek-v4-pro"]
```

```text
> @src/lib.rs what does this module do?
[Runie reads the file and explains it]

> refactor the error handling to use thiserror
[Runie proposes a diff]

/approve
[Runie applies the edit]

/model claude-sonnet-4
> now review the change
[Runie switches model and reviews]

/save refactor-errors
```

That is the whole loop: ask, inspect, approve, switch, save.

## Model support

Runie speaks the OpenAI-compatible API, so it works with most hosted and local providers. A few examples:

| Provider | Use it for |
|----------|------------|
| Anthropic | Long-context reasoning |
| OpenAI | General coding and fast responses |
| Google Gemini | Large context windows |
| DeepSeek | Cost-efficient coding |
| OpenRouter | One key, many providers |
| Groq / Fireworks / Together | Fast inference |
| Ollama | Local, private models |

Switch at runtime with `/model <provider>/<model>`. No silent fallbacks; if a provider is missing, Runie tells you immediately.

## Trust and control by default

Runie is designed around the idea that an agent should not touch your system unless you understand what it is doing.

- **Permission gating** for every write, shell command, and file edit.
- **Diff previews** before any file change is applied.
- **Read-only mode** (`/readonly`) when you only want answers, not edits.
- **Local config, local sessions**—your history is a JSONL file on disk, not a cloud log.

## Modes

| Mode | When to use | Command |
|------|-------------|---------|
| **TUI** | Interactive coding sessions | `./target/release/runie` |
| **Print** | Plain text output for scripts | `./target/release/runie print "..."` |
| **JSON** | Programmatic consumption | `./target/release/runie json "..."` |
| **Server** | RPC surface (experimental) | `./target/release/runie server` |

## Development

```bash
# Run with dev config and mock provider
./dev.sh

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt
```

Runie is tested in four layers: pure state logic, event handling, `TestBackend` rendering, and tmux smoke tests. See `AGENTS.md` for contributor conventions.

## Roadmap

- **R3** — Unify duplicated types, flatten the event system, complete the AppState refactor, and consolidate the TUI crates. See `tasks/` for details.
- **R4** — Solo and Team execution modes, model trait routing, and orchestrated multi-agent workflows. See `docs/adr/0020-team-mode-orchestration.md`.

## License

MIT
