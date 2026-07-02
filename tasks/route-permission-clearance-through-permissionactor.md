# Route permission clearance through `PermissionActor`

**Status**: done

## Description

`UiActor` clears `permission_request_mut()` directly after resolving a permission. Send the resolution to `PermissionActor` and wait for `PermissionRequestDismissed`.

## Acceptance criteria

- [x] **Unit tests** — Resolving a permission emits the dismissal event; only then is the projection cleared.
- [x] **E2E tests** — Permission grant/deny replay still works.
- [x] **Live tmux tests** — Approve/deny a tool permission in tmux and verify UI updates.

## Tests

### Unit tests
- Permission resolution event flow via `PermissionRequestDismissed`.

### E2E tests
- Replay fixture with permission prompts.

### Live tmux tests
- Run a tool that asks permission and respond to the prompt.

## Files touched

- `crates/runie-core/src/actors/permission/ractor_permission.rs` — emit `PermissionRequestDismissed` after resolving
- `crates/runie-tui/src/ui_actor.rs` — emit and apply `PermissionRequestDismissed` locally for synchronous state update

## Validation

1. `cargo test --workspace` passes
2. `cargo check --workspace` passes
3. `cargo clippy --workspace --lib --bins -- -D warnings` passes
