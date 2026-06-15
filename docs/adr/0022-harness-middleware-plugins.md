# Harness Skills (Plugin-Like Middleware)

## Context

Empirical harness-engineering work on CLI coding agents (LangChain deepagents-cli, Can Bölük’s oh-my-pi, custom MCP rigs) shows that model capability is only part of the story. For a fixed model, changes to the harness—edit-tool schemas, verification loops, context injection, loop detection, and tool descriptions—can move benchmark scores by 5–17 pp, cut tokens by ~20%, and eliminate whole classes of failures (doom loops, build breaks, missing tests).

Runie already defines a **Skill** in `docs/CONTEXT.md` as "a self-describing interceptor on the event bus." Currently, skills are mainly used for context injection. This ADR extends the Skill concept to cover harness-level middleware: each Skill is a default-on, configurable, togglable behavior that wraps or observes the agent turn.

## Decision

Runie will ship harness improvements as **Skills**: event-bus interceptors that are default-on, user-configurable, and can be disabled. The agent turn in `runie-agent/src/turn.rs` remains an orchestration shell; policy lives in Skills.

### Skill lifecycle hooks

Skills register against three hooks:

| Hook | When fired | Example skills |
|------|------------|----------------|
| `on_turn_start` | Before the LLM call | Startup context injector |
| `on_tool_call` | Before/after each tool execution | Loop detector, permission checker |
| `on_turn_end` | After the model declares completion | Verification loop |

### Configuration shape

```toml
[harness]
edit_tool = "hashline"        # "search_replace" | "hashline" | "apply_patch"

[harness.skills]
startup_context = { enabled = true }
loop_detector = { enabled = true, max_repeats = 3 }
verification_loop = { enabled = true, command = "cargo test" }
```

### First-party harness skills

1. **Hashline Edit Skill** — replaces exact-string `search`/`replace` with line-addressed edits using short content hashes. Modeled on the +17.5 pp result from oh-my-pi.
2. **Verification Loop Skill** — after the model claims completion, runs a configured verification command and feeds failures back for a fix pass.
3. **Startup Context Injector Skill** — runs a small set of discovery commands (`pwd`, tool detection, git status) and injects the result into the system prompt.
4. **Loop Detector Skill** — tracks recent tool calls and emits a recovery prompt when the agent appears stuck.
5. **Tool Schema Enricher Skill** — adds `examples` arrays to tool schemas to reduce tool-usage failures.

### Plugin vs Skill terminology

The requested behavior is "plugins: default but with possibility to change, turn off." Runie’s existing term for this is **Skill**. This ADR uses "Skill" everywhere; "plugin" is treated as a user-facing synonym for the same concept.

## Consequences

- **Positive:** Harness improvements become experiments users can run without forking the agent.
- **Positive:** The agent turn stays focused on orchestration; policy lives in Skills.
- **Positive:** Default-on Skills give new users strong baseline behavior.
- **Trade-off:** Skill ordering matters; the framework must define clear precedence and conflict rules.
- **Trade-off:** Each Skill adds test surface; Layer 1/2 tests are required per Skill.
