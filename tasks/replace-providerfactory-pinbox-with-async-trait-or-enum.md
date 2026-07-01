# Replace `ProviderFactory` `Pin<Box<dyn Future>>` with `async_trait` or provider enum

## Status

`todo`

## Description

`ProviderFactory` in `crates/runie-core/src/provider/provider_trait.rs` returns `Pin<Box<dyn Future<...>>>` from `validate_key`. This boilerplate can be removed with `async_trait` or by collapsing the factory into a provider enum.

## Acceptance criteria

1. **Unit tests** — Mock factory builds and validates keys correctly without manual `Pin<Box<...>>`.
2. **E2E tests** — A provider replay turn completes successfully with the refactored factory.
3. **Live run tests** — Switch provider/model in tmux and confirm the factory resolves credentials and builds the provider.

## Tests

### Unit tests
- Mock factory builds and validates keys correctly.

### E2E tests
- A replay turn with the refactored factory completes successfully.

### Live run tests
- In tmux, use `/provider` or model selection to switch providers and start a turn.
