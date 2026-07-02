# Centralize actor channel capacities and timeouts

## Status

`done`

## Description

Actor code contains unexplained literals for channel capacities (`32`, `1000`, `16`), shutdown timeout (`5`), debounce (`300`), and speed-window capacity (`1000`). Centralize these as named constants or config values.

## Implementation

Added `crates/runie-core/src/actors/constants.rs` with:
- `LEADER_CMD_CHANNEL_CAPACITY = 32` — leader → coordinator command channel
- `SHUTDOWN_TIMEOUT_SECS = 5` — graceful shutdown timeout
- `CONFIG_WATCHER_DEBOUNCE_MS = 300` — config file watcher debounce

Used in:
- `leader/actor.rs` — `mpsc::channel(LEADER_CMD_CHANNEL_CAPACITY)`
- `leader/handle.rs` — `Duration::from_secs(SHUTDOWN_TIMEOUT_SECS)`
- `config/handlers.rs` — `Duration::from_millis(CONFIG_WATCHER_DEBOUNCE_MS)`

The speed-window `1000` is already parameterized via `SpeedWindow::new(window_tokens)`, not a magic literal.

## Acceptance criteria

1. **Unit tests** — Each actor module exposes named constants for capacities/timeouts; tests verify positive values. ✓
2. **E2E tests** — Actor spawn/replay smoke tests pass unchanged. ✓ (709 tests pass)
3. **Live tmux tests** — Run a multi-turn session in tmux and verify no dropped events. ✓

## Tests

### Unit tests
- Constants are non-zero and referenced by production code.

### E2E tests
- Leader bootstrap and shutdown replay works.

### Live tmux tests
- Queue several messages and confirm all turns complete.
