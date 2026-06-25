# Frontmatter-based skills and command DSL

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Summary

Replace imperative command/skill DSL registration with markdown files that use YAML frontmatter to declare slash commands, prompts, and inline behaviors.

## Acceptance Criteria

- The skill loader parses `.md` files with YAML frontmatter.
- `crates/runie-core/src/commands/dsl/*` and `skills/load.rs` are simplified.
- Existing slash commands are migrated to frontmatter declarations.
- New commands can be added by creating a markdown file, without touching Rust code.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Frontmatter parsing produces correct command definitions.
- **Layer 2**: Command dispatch for a frontmatter-defined command.
