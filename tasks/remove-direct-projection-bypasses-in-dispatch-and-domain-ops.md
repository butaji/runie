# Remove direct projection bypasses in dispatch and domain ops

## Status

`todo`

## Description

Environment facts (`git_info`, `cwd_name`) are assigned directly to `AppState` projection fields in `dispatch.rs` and exposed via public setters in `domain_ops.rs`. These bypass the accessor/actor boundary.

Target locations:
- `crates/runie-core/src/update/dispatch.rs:306-309`
- `crates/runie-core/src/model/state/domain_ops.rs:14-22`

## Acceptance criteria

- `Event::EnvDetected` updates `AppState` through the projection accessor, not direct field write.
- `set_git_info`/`set_cwd_name` setters are removed or made private to tests.
- Production code uses only actor-emitted events to update these fields.

## Tests

### Layer 1 — State/Logic
- `AppState::apply_event(Event::EnvDetected { git_info, cwd_name })` updates the projection correctly.

### Layer 2 — Event Handling
- Dispatching `EnvDetected` through the central dispatcher updates only the intended projection accessors.
