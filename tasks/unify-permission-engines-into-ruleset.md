# Unify permission engines into a single ruleset

## Status

`todo`

## Context

Runie has two permission rule engines that are not actually unified:
- `PermissionManager` in `crates/runie-core/src/permissions/mod.rs:161-230` evaluates a `Vec<Box<dyn PermissionPolicy>>` built from `PermissionMode`.
- `PermissionSet` in `crates/runie-core/src/permissions/rules.rs:132-279` evaluates `PermissionRule` with scope precedence.
- `crates/runie-agent/src/actor.rs:161-178` hardcodes `GitTrackedWriteApprove` into the agent actor, but this policy is not part of `PermissionManager::build_policies`, so headless/TUI/config-driven paths get different defaults.

## Goal

Collapse both engines into a single declarative `PermissionSet`/`PermissionRule` engine. `PermissionMode` should produce a `PermissionSet`; `PermissionPolicy` becomes an internal optimization or is deleted.

## Acceptance Criteria

- [ ] Delete `PermissionManager` and `PermissionPolicy` trait, or reduce to a thin `PermissionSet` builder.
- [ ] `PermissionMode` produces a deterministic `PermissionSet`.
- [ ] Agent actor no longer injects special policies; all rules live in the ruleset.
- [ ] Every existing permission decision is byte-for-byte preserved.
- [ ] All permission tests pass.

## Design Impact

No change to TUI element design or composition. Only permission evaluation behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for each `PermissionMode` → expected ruleset and each rule’s allow/ask/deny decision.
- **Layer 2 — Event Handling:** `PermissionRequest`/`PermissionResponse` events are unchanged.
- **Layer 3 — Rendering:** `TestBackend` permission dialog snapshots match.
- **Layer 4 — E2E:** Provider replay fixture with denied/approved tools produces the same outcome.
- **Live tmux validation:** Start a turn that triggers a write tool and a git-tracked write; the same prompts/defaults appear as before.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
