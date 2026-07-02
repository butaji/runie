# Unify SSE parsing on `OpenAiFrame::from_line`

## Status

`todo`

## Description

`parse_sse_event`, `replay_sse`, and `OpenAiFrame::from_line` all parse the same SSE grammar. Consolidate live streaming and replay on a single frame parser.

## Acceptance criteria

1. **Unit tests** — Live and replay SSE lines produce identical parsed frames.
2. **E2E tests** — Provider replay fixtures still stream correctly.
3. **Live tmux tests** — Run a streaming turn in tmux and verify chunks are rendered.

## Tests

### Unit tests
- `data: {...}`, `data: [DONE]`, comments, and multiline events.

### E2E tests
- Replay SSE fixtures produce the same provider events.

### Live tmux tests
- Submit a prompt and watch streaming tokens appear.
