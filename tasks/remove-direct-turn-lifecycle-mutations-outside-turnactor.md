# Remove direct turn lifecycle mutations outside `TurnActor`

## Status

`todo`

## Description

Remove `AppState::start_turn`, `AppState::next_id`, `set_turn_active`, `set_streaming`, and token tracker configuration from direct mutation sites. Route them through `TurnActor`.

## Acceptance criteria

1. **Unit tests** — Direct lifecycle mutators are removed or gated to `#[cfg(test)]`.
2. **E2E tests** — Turn lifecycle events still work in replay.
3. **Live tmux tests** — Submit, abort, and complete turns in tmux.

## Tests

### Unit tests
- Grep confirms no direct mutators in production.

### E2E tests
- Lifecycle replay.

### Live tmux tests
- Submit and abort a turn.
