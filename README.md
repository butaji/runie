# Runie

<p align="center">
  <b>Stop letting your AI credits rot.</b><br>
  One terminal. Every model. Zero waste.
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#why-runie">Why</a> ·
  <a href="#features">Features</a> ·
  <a href="#the-loop">The loop</a> ·
  <a href="#providers">Providers</a> ·
  <a href="#development">Dev</a>
</p>

```bash
cargo build --release
./target/release/runie
```

---

## Why Runie

You pay for OpenAI, Anthropic, Groq, OpenRouter, Gemini, DeepSeek. Most of those credits sit unused while you default to whatever chat tab is already open.

Runie fixes the interface problem. It lives in your terminal, talks to every provider, and lets you swap models mid-task without losing context.

No browser. No paste dance. No wasted credits.

## What it does

|  |  |
|---|---|
| 🧠 **Multi-model routing** | Switch provider/model mid-session with `/model anthropic/claude-sonnet-4-6` |
| 🛡️ **Permission-gated tools** | `ReadFile`, `ListDir`, `WriteFile`, `EditFile`, `Bash`, `Grep`, `Find`, `FetchDocs` only with your approval |
| 💾 **Forkable sessions** | Save, load, fork, branch. Your history is local JSONL you own |
| ⚡ **Terminal-native** | Keyboard-driven TUI next to your editor |
| 🔄 **Scoped model cycling** | Rotate your configured shortlist with one hotkey |
| 🤖 **Team mode (R4)** | Orchestrator designs multi-agent workflows and routes roles to the best models |

## Features

### One session. Every model.

```text
> refactor error handling to use thiserror
[Runie proposes a diff]

/approve
[Runie applies it]

/model deepseek/deepseek-chat
> write a regression test
[Cheaper model writes the test]

/save retry-backoff-fix
```

Use the right brain for the right job—and finally spend the credits you already bought.

### Tools that ask first

Runie can read, list, write, edit, run shell commands, grep, find, and fetch docs—but never silently. Every write shows a diff. Every shell command asks. `/readonly` disables edits entirely.

### Sessions you own

- `/save refactor-auth`
- `/load refactor-auth`
- `/fork 12`
- `/tree`

Your context survives restarts, branches, and forks. It lives on your disk, not a server.

### Terminal commands

| Shortcut / command | Action |
|---|---|
| `Ctrl+P` | Open command palette |
| `Ctrl+L` | Switch model |
| `Shift+Tab` | Cycle thinking level |
| `@path/to/file` | Reference a file inline |
| `/` | Slash command palette |

## The loop

```text
> @src/lib.rs why does this retry never back off?
[explains the bug]

> rewrite with exponential backoff
[proposes diff]

/approve
[applies edit]

/model groq/llama-3.3-70b-versatile
> write a regression test
[switches model, writes test]

/save retry-backoff
```

Inspect. Edit. Approve. Route. Save.

## Quick start

```bash
# Build
cargo build --release

# Interactive TUI
./target/release/runie

# No API key? Use the mock provider
RUNIE_MOCK=1 ./target/release/runie

# One-shot CLI
./target/release/runie-print "find unused imports" < src/main.rs
```

Add keys in `~/.runie/config.toml`:

```toml
provider = "openai"
model = "gpt-4o"

[models]
scoped = [
  "gpt-4o",
  "anthropic/claude-sonnet-4-6",
  "deepseek/deepseek-chat",
]
```

Config hot-reloads while you type.

## Providers

Anthropic · OpenAI · Google Gemini · DeepSeek · OpenRouter · Groq · Fireworks · Together · MiniMax · Moonshot AI · xAI · Mistral · Ollama

If you have credits there, Runie helps you use them.

## Modes

| Mode | Command |
|---|---|
| TUI | `./target/release/runie` |
| Print | `./target/release/runie-print "..."` |
| JSON | `./target/release/runie-json "..."` |
| Server | `./target/release/runie-server` |

## Development

```bash
./dev.sh                 # mock provider
cargo test --workspace   # four-layer test suite
cargo clippy --workspace
cargo fmt
```

See `AGENTS.md` for conventions.

## Roadmap

- **R3** — Unify types, flatten events, finish state refactor, consolidate TUI crates. See `tasks/`.
- **R4** — Team mode: orchestrated multi-agent workflows. See `docs/adr/0020-team-mode-orchestration.md`.

## License

MIT

---

<details>
<summary><b>For robots and detail lovers</b></summary>

Runie is a terminal-native harness for LLM-powered coding agents. It is not a chat website and not tied to one provider. It is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

Most coding agents force you to leave your terminal: open a browser, paste context, watch a spinner, copy code back by hand. Every switch costs focus and context. Runie keeps you in the shell, next to your editor, with direct access to the codebase, shell, and git history.

Key differentiators:

- **Multi-model routing in one session** — pick the model that fits the task and the credits you want to spend.
- **Permission-gated tool execution** — real power, real control.
- **Scoped model cycling** — rotate a configured shortlist to compare answers in context.
- **Persistent, forkable sessions** — conversations become reusable assets.
- **Terminal-native workflow** — keyboard shortcuts, inline file references, fuzzy command palette.
- **Team mode (R4)** — orchestrator designs workflows of specialized roles and routes them to the best models.
- **Four layers of tests** — state logic, event handling, TestBackend rendering, and tmux smoke tests.

Runie is built for developers who refuse to leave the terminal and want every model they pay for to pull its weight.
</details>
