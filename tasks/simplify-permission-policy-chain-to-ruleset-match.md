# Simplify permission policy chain to a ruleset match

## Status

`todo`

## Description

The permission system uses a chain of policy objects (`DefaultToolApprove`, `GitTrackedWriteApprove`, `FileAccessAsk`, `PermissionSetPolicy`). This can be collapsed into a single ruleset evaluated by a plain `match`, or `tower` layers if ordering is truly required.

## Acceptance criteria

- Policy chain is replaced by a single ruleset + match evaluation.
- Existing behavior (default allow, git-tracked write approve, file access ask, user rules) is preserved.
- `PermissionGate` uses the simplified evaluator.

## Tests

### Layer 1 — State/Logic
- Ruleset evaluation returns the same action as the old chain for representative inputs.

### Layer 2 — Event Handling
- `PermissionRequest`/`PermissionResponse` events still flow correctly.
