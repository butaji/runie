# Use atomic writes for config and session files

## Status

`todo`

## Description

Config and session writes are not atomic; a crash can leave empty or partial files. Use `atomicwrites` or `tempfile::NamedTempFile::persist`.

## Acceptance criteria

1. **Unit tests** — A simulated crash mid-write leaves the original file intact.
2. **E2E tests** — Save config/session and reload; data is intact.
3. **Live tmux tests** — Save a session in tmux and resume it after restart.

## Tests

### Unit tests
- Atomic replacement under failure injection.

### E2E tests
- Save/reload round-trip.

### Live tmux tests
- Save session, kill process, resume.
