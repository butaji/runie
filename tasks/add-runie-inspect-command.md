# Add `runie inspect` command

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: frontmatter-based-skills-dsl
**Blocks**: none

## Summary

Add a `runie inspect` command that prints the runtime configuration discovered for the current directory. Because everything is declared in files, the system can show exactly what it loaded.

## Output sections

```bash
runie inspect
runie inspect --json
```

Human-readable sections:
- Project instructions (`AGENTS.md`)
- Loaded skills (user + bundled)
- Registered slash commands
- Built-in subagent types
- MCP servers
- Permission rules
- Config sources and layers
- Active actor states
- Model catalog entries
- Recent slash commands (MRU)

Secrets (API keys, tokens) are redacted in all output.

## Acceptance Criteria

- `runie inspect` prints a human-readable summary.
- `runie inspect --json` emits machine-readable JSON.
- The command is read-only and never mutates state or starts a turn.
- Secrets are redacted.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Config-source merge and redaction tests.
- **Layer 2**: `runie inspect` command emits the correct event and produces expected output.
