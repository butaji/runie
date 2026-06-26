# Declarative permission rules

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: permission-actor-owns-approvals
**Blocks**: none

## Summary

Replace imperative permission policy code with declarative allow/deny rules loaded from config, `AGENTS.md`, and CLI flags. `PermissionActor` evaluates the rule set against each tool call and emits `PermissionResolved` facts.

## Permission modes

| Mode | Behavior |
|---|---|
| `default` | Apply rules; ask when no rule matches. |
| `acceptEdits` | Auto-accept file edits; ask for shell commands. |
| `auto` | Auto-approve safe operations; ask for risky ones. |
| `dontAsk` | Approve unless a deny rule matches. |
| `bypassPermissions` | Approve everything (dangerous). |
| `plan` | Block write tools until a plan is approved. |

## Rule format

```toml
[[permissions]]
action = "allow"
tool = "read_file"

[[permissions]]
action = "deny"
tool = "bash"
pattern = "rm -rf /"

[[permissions]]
action = "ask"
tool = "write_file"
pattern = "*.rs"
scope = "project"
```

## Acceptance Criteria

- Rules support `allow`/`deny`/`ask` by tool name, file path pattern, shell command pattern, and scope (`user`, `project`, `session`).
- CLI flags `--allow`, `--deny`, `--tools`, and `--permission-mode` are supported.
- Rules are layered: built-in defaults → user config → project config (`AGENTS.md` / `.runie/config.toml`) → CLI flags.
- `PermissionActor` consumes a tool-call intent, evaluates rules, and emits `PermissionResolved`.
- The existing approval UI still handles `Ask` results.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Rule evaluation matrix (allow/deny/wildcard precedence).
- **Layer 2**: Permission event handling for CLI flag overrides.
