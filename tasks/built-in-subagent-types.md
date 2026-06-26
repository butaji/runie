# Built-in subagent types as declarative data

**Status**: todo
**Milestone**: R4
**Category**: Agent
**Priority**: P1

**Depends on**: leader-actor-shared-runtime
**Blocks**: none

## Summary

Ship built-in subagent types (`explore`, `plan`, `verify`, `check-work`) as declarative markdown files with YAML frontmatter. The subagent runner loads and executes them generically; users add custom types by dropping a file.

## File format

```markdown
---
name: explore
description: Fast codebase exploration for patterns and architecture.
prompt_mode: full
model: inherit
permission_mode: default
agents_md: true
---

You are an expert explorer. Search broadly, then narrow down. Use absolute paths.
Never create files unless explicitly requested.
```

Frontmatter fields:
- `name` — subagent type id.
- `description` — when to spawn this subagent.
- `prompt_mode` — `full` or `compact`.
- `model` — concrete model id, `inherit`, or `fast` trait.
- `permission_mode` — `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`.
- `agents_md` — whether to inject project `AGENTS.md` into context.

The markdown body is the prompt template. Variables are interpolated with `{{variable}}`.

## Acceptance Criteria

- Subagent type files live under `resources/agents/` (bundled defaults) and `~/.runie/agents/` (user overrides).
- A manifest with SHA-256 checksums validates bundled resources at build time.
- `SubagentRegistry` loads all types at startup and emits `AgentTypeRegistered` facts.
- Existing subagent runner dispatches through these definitions.
- Users can add a custom subagent type by creating a file; no Rust code changes.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Parse subagent type files, validate checksums, interpolate variables.
- **Layer 4**: Provider-replay test that spawns a built-in `explore` subagent.
