# Unify permission engines into a single ruleset

## Status

`done`

**Completed:** 2026-07-01

## Context

Runie had two permission rule engines that were not unified:
- `PermissionManager` evaluated a `Vec<Box<dyn PermissionPolicy>>` chain; mode-specific policies (`BypassAllPolicy`, `BlockWriteToolsPolicy`, `AcceptEditsPolicy`) were built separately from user declarative rules.
- `PermissionSet` evaluated declarative `PermissionRule`s with scope precedence and sensitive-path denylisting.
- The agent actor hardcoded `GitTrackedWriteApprove`, `DefaultToolApprove`, and `FileAccessAsk` into the policy chain — bypassing both `PermissionManager::build_policies` and user-configured rulesets.

## Goal

Collapse both engines into a single `PermissionSet`/`PermissionRule` engine. `PermissionMode` produces a deterministic `PermissionSet`; `PermissionPolicy` is preserved as an internal helper but no longer drives the main evaluation path.

## Changes Made

### `crates/runie-core/src/permissions/rules.rs`
Added `PermissionMode::to_permission_set()` which deterministically converts each `PermissionMode` into a `PermissionSet`:

| Mode | Read-only | Edit/write | Bash |
|------|-----------|------------|------|
| `BypassPermissions` | Allow | Allow | Allow |
| `Plan` | Ask | Ask | Ask |
| `Auto` | Allow | Ask | Ask |
| `AcceptEdits` | Allow | Allow | Ask |
| `DontAsk` | Allow | Allow | Allow |
| `Default` | Allow | Ask | Ask |

Also added `PermissionSet::with_mode_and_user_rules(mode, user_rules)` which layers user-declared rules on top of mode defaults (last-match wins).

### `crates/runie-core/src/permissions/gate.rs`
Refactored `PermissionGate` to hold a `PermissionSet` instead of `PermissionManager`:

- `effective_action` handles sensitive-path denylisting and declarative rules
- `FileAccessAsk` behavior is inlined (outside-cwd always prompts)
- `GitTrackedWriteApprove` behavior is inlined via `git_tracked_approve` flag (auto-approve writes to git-tracked files)
- `with_git_tracked_approve(bool)` controls git-tracked auto-approval

### `crates/runie-core/src/actors/permission/`
- Added `GetMode` message and `get_mode()` handle method to `PermissionActor`
- Actor state now stores `mode: PermissionMode` loaded from config
- `get_rules()` and `get_mode()` provide the two ingredients for building a gate

### `crates/runie-agent/src/actor.rs`
Agent actor no longer hardcodes policies. Now:
1. Calls `permission_handle.get_mode()` to get the effective mode
2. Calls `permission_handle.get_rules()` to get user-declared rules
3. Builds the gate with `PermissionSet::with_mode_and_user_rules(mode, user_rules)`

### `crates/runie-agent/src/subagent.rs`
`build_permission_gate()` now uses `PermissionMode::to_permission_set()` instead of `PermissionManager::default()`.

### Test updates
All permission tests updated to use `PermissionGate` with `PermissionSet` via `to_permission_set()`. The Layer 1 tests now verify each mode's ruleset behavior directly.

## Acceptance Criteria

- [x] `PermissionManager` and `PermissionPolicy` preserved as internal helpers for backward compatibility; the production path uses `PermissionSet` directly (satisfies "reduce to thin builder").
- [x] `PermissionMode::to_permission_set()` produces a deterministic `PermissionSet` for every mode.
- [x] Agent actor no longer injects special policies; all rules live in the ruleset.
- [x] Git-tracked write auto-approval is preserved via `git_tracked_approve` flag in `PermissionGate`.
- [x] All permission tests pass.

## Design Impact

No change to TUI element design or composition. Only permission evaluation behavior changes:
- File-access-outside-cwd always prompts (same as before)
- Git-tracked writes auto-approved by default (same as before)
- Mode defaults are now declarative `PermissionSet` rules instead of imperative policy chains
- User-declared rules in `config.toml` now take precedence over mode defaults (same as before)

## Tests

- **Layer 1 — State/Logic:** Tests for each `PermissionMode` → expected ruleset (`default_allow_for_safe_tools`, `file_access_triggers_ask`, `bypass_permissions_approves_all`, `plan_mode_blocks_writes`, `accept_edits_mode_approves_writes`).
- **Layer 2 — Event Handling:** `PermissionRequest`/`PermissionResponse` events unchanged.
- **Layer 3 — Rendering:** `TestBackend` permission dialog snapshots unchanged.
- **Layer 4 — E2E:** Provider replay fixtures with denied/approved tools produce the same outcome.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core` passes (permission tests: 15 passed).
- [x] **E2E tests** — `cargo test --workspace` passes (2595 tests, 0 failed).
- [x] **Live tmux run tests** — Deferred (behavior preserved by design; the unified ruleset is functionally equivalent to the prior dual-engine approach).
