# Unify SSE parsing on `OpenAiFrame::from_line`

## Status

`done`

## Description

`parse_sse_event`, `replay_sse`, and `OpenAiFrame::from_line` all parse the same SSE grammar. Consolidate live streaming and replay on a single frame parser.

## Acceptance criteria

- [x] **Unit tests** — Live and replay SSE lines produce identical parsed frames.
- [x] **E2E tests** — Provider replay fixtures still stream correctly.
- [x] **Live tmux tests** — Run a streaming turn in tmux and verify chunks are rendered.

## Implementation

Extracted `parse_sse_line(line: &str) -> Option<Result<OpenAiFrame, ModelError>>` as the shared parser used by both `replay_sse` and `stream_sse_events`. Removed the intermediate `SseEvent`/`Chunk`/`Delta`/`ToolCallDelta` types from `stream.rs` (they now live in `protocol.rs` alongside their usage). Deleted `parse_sse_event` (dead-weight wrapper) and rewrote its two tests to use `OpenAiFrame::from_line` directly. Renamed tests to `openai_frame_parses_text_delta` and `openai_frame_parses_done`. Both `stream_sse_events` and `replay_sse` now call `OpenAiFrame::from_line` and `protocol.step()` directly — no intermediate type conversions.

## Tests

### Unit tests
- `data: {...}`, `data: [DONE]`, comments, and multiline events.

### E2E tests
- Replay SSE fixtures produce the same provider events.

### Live tmux tests
- Submit a prompt and watch streaming tokens appear.
