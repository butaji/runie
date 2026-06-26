# AGENTS.md and frontmatter-based declarative configuration

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: leader-actor-shared-runtime
**Blocks**: add-runie-inspect-command

## Summary

Replace imperative command/skill DSL registration with markdown files that use YAML frontmatter to declare slash commands, skills, agent profiles, hooks, and behaviors.

## Skill format

```markdown
---
name: check-work
description: Verify changes with a subagent.
metadata:
  short-description: "Verify changes with a subagent"
triggers:
  - command: /check-work
  - command: /verify
  - file: "*.xlsx"
---

## Usage

`/check-work [focus area]`

## Steps

1. Spawn a verifier subagent.
2. Read the verdict.
3. Fix issues if `VERDICT: FAIL`.
```

## Command format

```yaml
# .runie/commands/bookmark.yaml
name: bookmark
description: Bookmark the current assistant message.
intent: BookmarkMessage
shortcut: Ctrl+b
```

## Acceptance Criteria

- The loader parses `.md` files with YAML frontmatter for skills and `.yaml` files for simple commands.
- `commands/dsl/*` and `skills/load.rs` are simplified to a generic loader.
- Triggers include slash commands, file patterns, and explicit user invocation.
- Existing slash commands are migrated to frontmatter declarations.
- New commands/skills can be added by creating a file, without touching Rust code.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Frontmatter parsing produces correct definitions and triggers.
- **Layer 2**: Command dispatch for a frontmatter-defined command.
