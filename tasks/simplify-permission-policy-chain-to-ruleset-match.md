# Simplify permission policy chain to a ruleset match

## Status

`todo`

## Description

The permission system uses a chain of policy objects (`DefaultToolApprove`, `GitTrackedWriteApprove`, `FileAccessAsk`, `PermissionSetPolicy`). This can be collapsed into a single ruleset evaluated by a plain `match`, or `tower` layers if ordering is truly required.

## Acceptance criteria

1. **Unit tests** — Ruleset evaluation returns the same action as the old chain for representative inputs.
2. **E2E tests** — `PermissionRequest`/`PermissionResponse` events still flow correctly through the actor.
3. **Live run tests** — Exercise allow, deny, and auto-approve paths in tmux and confirm the same behavior.

## Tests

### Unit tests
- Ruleset evaluation returns the same action as the old chain for representative inputs.

### E2E tests
- `PermissionRequest`/`PermissionResponse` events still flow correctly.

### Live run tests
- Run a tool that triggers permission prompts in tmux; test allow, deny, and remember-choice flows.
