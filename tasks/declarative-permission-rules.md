# Declarative permission rules

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: permission-actor-owns-approvals
**Blocks**: none

## Summary

Replace imperative permission policy code with declarative allow/deny rules loaded from config, `AGENTS.md`, and CLI flags. `PermissionActor` evaluates the rule set against each tool call and emits `PermissionResolved` facts.

## Acceptance Criteria

- Rule format supports `allow`/`deny` by tool name, file path pattern, shell command pattern, and scope (`user`, `project`, `session`).
- CLI flags `--allow`, `--deny`, `--tools`, and `--permission-mode` are supported.
- Rules are layered: built-in defaults → user config → project config → CLI flags.
- `PermissionActor` consumes a tool-call intent, evaluates rules, and emits `PermissionResolved`.
- The existing approval UI still handles `Ask` results.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Rule evaluation matrix (allow/deny/wildcard precedence).
- **Layer 2**: Permission event handling for CLI flag overrides.
