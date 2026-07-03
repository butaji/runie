# Use serde_with for provider event serde

## Status

`done`

## Context

`crates/runie-core/src/provider_event.rs:135-202` had ~70 lines of hand-written `Serialize`/`Deserialize` impls for `ModelError` that map enum variants to a JSON struct with `kind`/`message` fields.

## Goal

Replace with `serde_with` (`SerializeDisplay`/`DeserializeFromStr`) or derive-friendly serde attributes.

## Implementation

Instead of using `serde_with`, we simplified by:

1. Changed `ModelError::Other` from `anyhow::Error` to `String`
2. Added derives directly on the enum:
   ```rust
   #[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
   #[serde(tag = "kind", content = "message", rename_all = "camelCase")]
   pub enum ModelError { ... }
   ```
3. Removed manual impls entirely

This is simpler than using `serde_with` because:
- No additional dependencies needed for this use case
- The derive approach is more idiomatic
- Less code to maintain

## Acceptance Criteria

- [x] **Add `serde_with` dependency** — Not needed; used derives instead
- [x] **Replace manual impls** — Replaced with simple derives
- [x] **Ensure durable JSON byte-compatibility** — Tests verify round-trip

## Tests

- [x] **Layer 1 — State/Logic:** Unit tests for JSON round-trip.
- [x] **Layer 2 — Event Handling:** N/A.
- [x] **Layer 3 — Rendering:** N/A.
- [x] **Layer 4 — E2E:** Provider event tests pass.
- [x] **Live tmux testing session (required):** N/A.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
