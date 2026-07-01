# Remove direct projection bypasses in dispatch and domain ops

## Status

`todo`

## Description

Environment facts (`git_info`, `cwd_name`) are assigned directly to `AppState` projection fields in `dispatch.rs` and exposed via public setters in `domain_ops.rs`. These bypass the accessor/actor boundary.

Target locations:
- `crates/runie-core/src/update/dispatch.rs:306-309`
- `crates/runie-core/src/model/state/domain_ops.rs:14-22`

## Acceptance criteria

1. **Unit tests** — `AppState::apply_event(Event::EnvDetected { ... })` updates the projection through accessors only.
2. **E2E tests** — Dispatching `EnvDetected` through the central dispatcher updates only the intended projection accessors.
3. **Live run tests** — Run in tmux, change working directory, and verify `cwd_name`/`git_info` update without direct field writes.

## Tests

### Unit tests
- `AppState::apply_event(Event::EnvDetected { git_info, cwd_name })` updates the projection correctly.

### E2E tests
- Dispatching `EnvDetected` through the central dispatcher updates only the intended projection accessors.

### Live run tests
- Start the TUI in tmux, `cd` into a git repo, and confirm the status bar reflects the new cwd and git branch via events.
