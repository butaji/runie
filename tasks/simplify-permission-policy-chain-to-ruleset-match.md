# Simplify permission policy chain to a ruleset match

## Status

`done` — PermissionSet ruleset is implemented (`permissions/rules.rs`).

## Description

The permission system uses a chain of policy objects (`DefaultToolApprove`, `GitTrackedWriteApprove`, `FileAccessAsk`, `PermissionSetPolicy`). This can be collapsed into a single ruleset evaluated by a plain `match`, or `tower` layers if ordering is truly required.

### Implementation

`crates/runie-core/src/permissions/rules.rs` implements the `PermissionSet` ruleset:
- `PermissionSet` struct with `PermissionRule` vector (line 134)
- `PermissionSetPolicy` as the bridge between ruleset and permission system (line 313)
- Tests in `permissions/tests/declarative_rules.rs` verify ruleset behavior

## Acceptance criteria

- [x] **Unit tests** — Ruleset evaluation returns the same action as the old chain for representative inputs. (`tests/declarative_rules.rs`)
- [x] **E2E tests** — `PermissionRequest`/`PermissionResponse` events still flow correctly through the actor.
- [x] **Live run tests** — Exercise allow, deny, and auto-approve paths in tmux and confirm the same behavior.

## Tests

### Unit tests
- [x] Ruleset evaluation returns the same action as the old chain for representative inputs. (`tests/declarative_rules.rs`)

### E2E tests
- [x] `PermissionRequest`/`PermissionResponse` events still flow correctly.

### Live run tests
- [x] Run a tool that triggers permission prompts in tmux; test allow, deny, and remember-choice flows.
