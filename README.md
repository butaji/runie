# Runie

> **Stop letting your AI credits rot.**
>
> Runie is the terminal-native coding harness that routes every task to the right model, executes tools with your permission, and turns scattered API credits into shipped code.

```bash
cargo build --release
./target/release/runie
```

---

## The quiet waste nobody talks about

You have OpenAI credits. Anthropic credits. Groq, OpenRouter, DeepSeek, maybe Gemini. Some are expiring. Most are sitting there while you default to whatever chat window is already open.

That is not a preference problem. It is an interface problem.

Every coding assistant forces you into its lane: one provider, one model, one conversation, one browser tab. Switching costs more than the click. It costs context, history, and the mental map of what you were doing. So you stay in the lane. And your other credits quietly evaporate.

Runie removes the lane.

## One terminal. Every model. Zero waste.

Runie is a keyboard-driven command center for AI-assisted coding. It lives in your terminal, reads your actual files, runs real shell commands, and lets you swap models mid-task without losing the thread.

No browser. No paste dance. No copy-paste diff tennis. Just you, your codebase, and every model you have access to—routed by task, not by habit.

## What makes Runie different

Most agents ask you to adapt to them. Runie adapts to you.

| Ordinary agents | Runie |
|-----------------|-------|
| One provider, one model per chat | Any provider, any model, swapped with one command |
| Cloud history you cannot see | Sessions as local JSONL files you own |
| Pasted code, guessed context | Direct file reads, edits, grep, bash inside your repo |
| Binary trust or no tools | Per-action permission gating with diff previews |
| One-shot chat with no memory | Fork, branch, save, and resume conversations |
| GUI you leave to use your editor | Terminal UI that stays next to your editor |

## Features that change how you work

### Multi-model routing in one session

Type `/model openai/gpt-4o` for speed. Switch to `/model anthropic/claude-sonnet-4` for reasoning. Use `/model groq/llama-3.3-70b` for cheap inference. The same conversation keeps its context; only the brain changes.

**Why it matters:** You finally use the model that fits the job—and the credits you already paid for.

### Permission-gated tool execution

Runie can `read_file`, `write_file`, `edit_file`, `bash`, `grep`, `find`, and `ls`. But it cannot do any of them without your say. Every write shows a diff first. Every shell command asks once. Every approval can be scoped to the session.

**Why it matters:** The agent gets real power, but you keep real control.

### Scoped model cycling

Configure a shortlist of models under `[models.scoped]` in `~/.runie/config.toml` and cycle them with a hotkey. Compare how three different models answer the same prompt without leaving the TUI.

**Why it matters:** Stop guessing which model is best. Test them in context.

### Persistent, forkable sessions

Save a conversation with `/save refactor-auth`. Resume it tomorrow with `/load refactor-auth`. Fork it from any message with `/fork 12`. Branch it with `/tree`. Your history is a structured JSONL file on disk, not a server log you cannot export.

**Why it matters:** Your best prompts and reasoning become reusable assets.

### Terminal-native workflow

- `@path/to/file` references files inline.
- `/` commands open a fuzzy palette.
- `Ctrl+P` jumps to any command.
- `Ctrl+L` switches models.
- `Shift+Tab` cycles thinking levels.
- Config hot-reloads while you edit `~/.runie/config.toml`.

**Why it matters:** You never leave the keyboard flow you already trained for.

### Team mode (R4)

Solo mode is one agent, one model. Team mode spins up an orchestrator that designs a workflow of specialized roles, routes each role to the best connected model, and executes steps in parallel or sequence. One human prompt becomes a coordinated team of agents.

**Why it matters:** Hard problems get decomposed automatically and executed by the right models for each piece.

### Four layers of tests

Runie is built under a strict test discipline: pure state logic, event handling, `TestBackend` rendering, and tmux smoke tests. Features do not ship without coverage.

**Why it matters:** A harness that breaks while you are relying on it is worse than useless.

## The cred-maximization loop

```toml
# ~/.runie/config.toml
provider = "openai"
model = "gpt-4o"

[models]
scoped = [
  "gpt-4o",
  "anthropic/claude-sonnet-4",
  "groq/llama-3.3-70b-versatile",
  "deepseek-v4-pro",
]
```

```text
> @src/lib.rs why does this retry logic never back off?
[Runie reads the file and explains the bug]

> rewrite it with exponential backoff
[Runie proposes a diff]

/approve
[Runie applies the edit]

/model deepseek-v4-pro
> write a regression test for this
[Runie switches to the cheaper model and writes the test]

/save retry-backoff-fix
```

That is the loop: inspect, edit, approve, route to the right model, save. No tab switching. No credit waste.

## Supported providers

Runie speaks the OpenAI-compatible API, so it connects to almost every hosted and local provider:

| Provider | What you use it for |
|----------|---------------------|
| Anthropic | Long-context reasoning, careful code review |
| OpenAI | General coding, fast responses |
| Google Gemini | Huge context windows |
| DeepSeek | Cost-efficient deep coding |
| OpenRouter | One key, many backends |
| Groq / Fireworks / Together | Low-latency inference |
| Ollama | Private, local models |

If you have credits there, Runie helps you spend them on actual work.

## Modes

| Mode | When to use | Command |
|------|-------------|---------|
| **TUI** | Interactive coding sessions | `./target/release/runie` |
| **Print** | Scriptable plain-text output | `./target/release/runie print "..."` |
| **JSON** | Programmatic consumption | `./target/release/runie json "..."` |
| **Server** | RPC surface (experimental) | `./target/release/runie server` |

## Quick start

```bash
# Build
cargo build --release

# Launch the TUI
./target/release/runie

# Try it without an API key
RUNIE_MOCK=1 ./target/release/runie

# One-shot CLI mode
./target/release/runie run "find all unused imports" < src/main.rs
```

Add your API keys through the onboarding flow or directly in `~/.runie/config.toml`. Your config is hot-reloadable; change it and keep typing.

## Built for developers who refuse to leave the terminal

Runie is not another chat app trying to replace your editor. It is the missing control layer between you, your codebase, and every model you have access to.

If you have been paying for AI credits and only using one provider because switching is too annoying, Runie fixes that.

If you have been copying code out of browser tabs and pasting it back, Runie fixes that.

If you have been starting from zero context every time you open a new chat, Runie fixes that.

## Development

```bash
./dev.sh                    # Run with mock provider
cargo test --workspace      # Four-layer test suite
cargo clippy --workspace    # Lint
cargo fmt                   # Format
```

See `AGENTS.md` for contributor conventions.

## Roadmap

- **R3** — Consolidate the codebase: unify duplicated types, flatten the event system, finish the state refactor, and merge `runie-term` into `runie-tui`. See `tasks/`.
- **R4** — Team mode: orchestrated multi-agent workflows with model trait routing. See `docs/adr/0020-team-mode-orchestration.md`.

## License

MIT
