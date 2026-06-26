# Declarative permission rules

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: permission-actor-owns-approvals
**Blocks**: none

## Summary

Replace imperative permission policy code with declarative allow/deny rules loaded from config, `AGENTS.md`, and CLI flags. `PermissionActor` evaluates the rule set against each tool call and emits `PermissionResolved` facts.

## Permission modes

| Mode | Behavior |
|------|----------|
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

- [x] Rules support `allow`/`deny`/`ask` by tool name, file path pattern, shell command pattern, and scope (`user`, `project`, `session`).
- [x] Permission modes implemented: `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`.
- [x] `PermissionRule` struct updated with `tool`, `path`, `pattern`, `scope` fields.
- [x] `PermissionScope` enum added for user/project/session scope.
- [x] `PermissionMode` enum extended with all modes.
- [x] `PermissionSet` supports layered evaluation with scope precedence.
- [x] Config supports `permissions` section with mode and rules.
- [x] JSON schema generated and validated.
- [x] `cargo check --workspace` is green.

## Tests

### Layer 1 — State/Logic
- [x] `permission_rule_with_path` — path pattern matching
- [x] `permission_rule_with_pattern` — command pattern matching
- [x] `permission_rule_with_scope` — scope assignment
- [x] `permission_set_with_scope_precedence` — session > project > user
- [x] `permission_set_filters_by_max_scope` — max scope limits
- [x] `permission_set_rules_for_scope` — filter by scope
- [x] `permission_set_extend` — rule set combination
- [x] `accept_edits_rules_auto_approve_file_edits` — acceptEdits mode
- [x] `dont_ask_rules_allow_all_except_deny` — dontAsk mode
- [x] `permission_mode_bypasses_all` — bypassPermissions mode
- [x] `permission_mode_requires_plan` — plan mode
- [x] `permission_mode_auto_approves_edits` — acceptEdits behavior
- [x] `permission_mode_auto_approves_safe` — auto behavior
- [x] `permission_mode_serialization` — JSON serialization
- [x] `rule_evaluation_matrix_allow_deny_ask` — rule precedence
- [x] `rule_evaluation_with_wildcard_patterns` — glob patterns
- [x] `rule_evaluation_complex_pattern` — bash command patterns

### Layer 2 — Event Handling
- N/A — rule evaluation, not event handling.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files Changed

- `crates/runie-core/src/permissions/mod.rs` — extended `PermissionMode` enum, added docs
- `crates/runie-core/src/permissions/rules.rs` — added `PermissionScope`, updated `PermissionRule`, `PermissionSet` with scope and pattern support
- `crates/runie-core/src/permissions/tests.rs` — updated existing tests, added `PermissionScope` import
- `crates/runie-core/src/permissions/tests/declarative_rules.rs` — new test file for declarative rules
- `crates/runie-core/src/config.rs` — added `PermissionsSection`
- `crates/runie-core/src/config/validate.rs` — added `validate_permissions` and helpers
- `crates/runie-core/src/config/schema.rs` — (regenerated)
- `crates/runie-agent/src/tests/permissions.rs` — updated tests for new API
- `config.schema.json` — regenerated
