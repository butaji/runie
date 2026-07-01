# Replace `ProviderFactory` `Pin<Box<dyn Future>>` with `async_trait` or provider enum

## Status

`todo`

## Description

`ProviderFactory` in `crates/runie-core/src/provider/provider_trait.rs` returns `Pin<Box<dyn Future<...>>>` from `validate_key`. This boilerplate can be removed with `async_trait` or by collapsing the factory into a provider enum.

## Acceptance criteria

- `ProviderFactory::validate_key` is an `async fn` (via `async_trait`) or the factory becomes an enum.
- All implementations compile without manual `Pin<Box<...>>`.

## Tests

### Layer 1 — State/Logic
- Mock factory builds and validates keys correctly.

### Layer 4 — Provider Replay / Mock-Tool E2E
- A replay turn with the refactored factory completes successfully.
