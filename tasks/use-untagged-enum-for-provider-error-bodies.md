# Use an untagged enum for provider error bodies

## Status

`todo`

## Description

`ErrorBodyJson` uses optional fields and accessor methods. A `#[serde(untagged)]` enum (`WrappedError` | `FlatError`) makes the schema explicit and removes helpers.

## Acceptance criteria

1. **Unit tests** — Both wrapped (`error.message`) and flat (`message`) bodies deserialize correctly.
2. **E2E tests** — Provider error replay fixtures still parse.
3. **Live tmux tests** — Trigger provider errors in tmux and confirm message extraction.

## Tests

### Unit tests
- Wrapped and flat error JSON deserialization.

### E2E tests
- Replay fixtures with both error shapes.

### Live tmux tests
- Use invalid credentials and check the error dialog.
