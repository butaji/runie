# Route permission clearance through `PermissionActor`

## Status

`todo`

## Description

`UiActor` clears `permission_request_mut()` directly after resolving a permission. Send the resolution to `PermissionActor` and wait for `PermissionRequestDismissed`.

## Acceptance criteria

1. **Unit tests** — Resolving a permission emits the dismissal event; only then is the projection cleared.
2. **E2E tests** — Permission grant/deny replay still works.
3. **Live tmux tests** — Approve/deny a tool permission in tmux and verify UI updates.

## Tests

### Unit tests
- Permission resolution event flow.

### E2E tests
- Replay fixture with permission prompts.

### Live tmux tests
- Run a tool that asks permission and respond to the prompt.
