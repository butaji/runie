# Simplify `ModelError` serde by storing message string

## Status

`done`

## Context

`ModelError::Other` wrapped `anyhow::Error`, forcing manual `Clone`/`Serialize`/`Deserialize` impls.

## Goal

Store `message: String` instead and derive the traits.

## Implementation

Changed `ModelError::Other` from:
```rust
#[error(transparent)]
Other(anyhow::Error),
```

To:
```rust
#[error("{0}")]
Other(String),
```

Added derives for `Clone`, `PartialEq`, `Serialize`, `Deserialize` on the enum:
```rust
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum ModelError {
    // ...
    #[error("{0}")]
    Other(String),
}
```

Removed:
- Manual `Clone` impl
- Manual `PartialEq` impl
- Manual `Serialize` impl
- Manual `Deserialize` impl
- `serde_with` helpers (`SerializeAs`/`DeserializeAs`)
- `ModelErrorJsonSchema` struct

Updated `From` impls to convert to string:
```rust
impl From<ProviderError> for ModelError {
    fn from(e: ProviderError) -> Self {
        ModelError::Other(e.to_string())
    }
}
```

## Files Changed

- `crates/runie-core/src/provider_event.rs` — simplified `ModelError` enum
- `crates/runie-core/src/event/from_provider_event.rs` — updated `ModelError::Other` usage
- `crates/runie-agent/src/tests/think_filter.rs` — updated test
- `crates/runie-provider/src/openai/stream.rs` — updated `ModelError::Other` usage

## Acceptance Criteria

- [x] **Unit tests** — `ModelError` round-trips through JSON and implements `Clone`/`PartialEq` via derives.
- [x] **E2E tests** — Provider replay with an error response serializes the same way.
- [x] **Live tmux tests** — Provoke a provider error in tmux and verify the displayed message is preserved.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
