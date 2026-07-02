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
