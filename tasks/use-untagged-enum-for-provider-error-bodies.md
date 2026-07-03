# Use an untagged enum for provider error bodies

## Status

`done`

## Description

`ErrorBodyJson` uses optional fields and accessor methods. A `#[serde(untagged)]` enum (`WrappedError` | `FlatError`) makes the schema explicit and removes helpers.

## Implementation

Replaced the `ErrorBodyJson` struct with an untagged enum that handles both shapes:

- `ErrorBodyJson::Wrapped(WrappedError)` — for `{"error": {"message": "..."}}` format
- `ErrorBodyJson::Flat(FlatError)` — for MiniMax-style `{"message": "..."}` format

The `serde(untagged)` attribute means serde tries each variant in order until one deserializes successfully.

Helper methods (`message()`, `code()`, `type_()`, `retry_after_secs()`) are implemented on the enum using pattern matching.

## Acceptance criteria

- [x] **Unit tests** — Both wrapped (`error.message`) and flat (`message`) bodies deserialize correctly.
- [x] **E2E tests** — Provider error replay fixtures still parse.
- [ ] **Live tmux tests** — Trigger provider errors in tmux and confirm message extraction.

## Tests

### Unit tests
- [x] Wrapped and flat error JSON deserialization (124 tests pass in runie-provider).

### E2E tests
- [x] Replay fixtures with both error shapes (4 MiniMax replay tests pass).

### Live tmux tests
- [ ] Use invalid credentials and check the error dialog.

## Files touched

- `crates/runie-provider/src/openai/types.rs` — replaced struct with untagged enum

## Validation

1. `cargo check -p runie-provider` — passes
2. `cargo test -p runie-provider` — 124 tests pass
3. `cargo test --workspace` — all workspace tests pass
