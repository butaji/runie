# Add `runie inspect` command

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: frontmatter-based-skills-dsl
**Blocks**: none

## Summary

Add a `runie inspect` command that prints the runtime configuration discovered for the current directory: loaded config layers, project instructions, skills, commands, MCP servers, permission rules, built-in subagents, and actor health.

## Acceptance Criteria

- `runie inspect` prints a human-readable summary of discovered configuration.
- `runie inspect --json` emits machine-readable JSON.
- Output includes: project instructions (`AGENTS.md`), loaded skills, registered commands, MCP servers, permission rules, built-in subagent types, config sources, and active actor states.
- Secrets (API keys, tokens) are redacted.
- The command is read-only and never mutates state or starts a turn.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Config-source merge and redaction tests.
- **Layer 2**: `runie inspect` command emits the correct event and produces expected output.
