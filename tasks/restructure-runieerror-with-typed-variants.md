# Restructure `RunieError` with typed variants

## Status

`todo`

## Description

`RunieError` currently wraps `anyhow::Error` and its `RunieErrorKind` is unused. Restructure it into a real central enum (e.g., `Permission`, `Config`, `Provider`, `Io`, `Validation`) or delete it.

## Acceptance criteria

1. **Unit tests** — Every variant round-trips through display/error-chain and maps to the correct kind.
2. **E2E tests** — Actor/provider error events carry the typed error structure.
3. **Live tmux tests** — Trigger errors in tmux and verify messages remain useful.

## Tests

### Unit tests
- Variant construction and `source()` chain.

### E2E tests
- Provider/config errors produce expected typed events.

### Live tmux tests
- Submit with invalid config and read the error dialog.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (error type change; actors remain authoritative).
- [ ] **Trigger events:** N/A (error type change doesn't introduce state transitions).
- [ ] **Observer events:** Typed errors are part of events.
- [ ] **No direct mutations:** N/A (error type change doesn't change state ownership).
- [ ] **No new mirrors:** N/A (error type change doesn't introduce new state).
- [ ] **Async work observed:** N/A (error type change doesn't introduce async work).
