# Simplify `ModelError` serde by storing a message string

## Status

`todo`

## Description

`ModelError::Other` wraps `anyhow::Error`, forcing manual `Clone`/`Serialize`/`Deserialize` impls. Store `message: String` instead and derive the traits.

## Acceptance criteria

1. **Unit tests** — `ModelError` round-trips through JSON and implements `Clone`/`PartialEq` via derives.
2. **E2E tests** — Provider replay with an error response serializes the same way.
3. **Live tmux tests** — Provoke a provider error in tmux and verify the displayed message is preserved.

## Tests

### Unit tests
- JSON round-trip and clone equality.

### E2E tests
- Replay fixture containing a provider error.

### Live tmux tests
- Disconnect network or use invalid key and check error display.
