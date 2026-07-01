# Fix leader TCP UTF-8 split with LinesCodec

## Status

`done`

## Context

`crates/runie-core/src/actors/leader/actor.rs:232-275` reads fixed 1024-byte chunks and converts each chunk with `std::str::from_utf8`, silently dropping multi-byte UTF-8 characters split across reads. It also manually buffers lines.

## Goal

Replace the manual byte buffering with `tokio::io::BufReader::lines()` or `tokio_util::codec::{FramedRead, LinesCodec}`. Share the same framing layer with the CLI transport.

## Acceptance Criteria

- [ ] Use `LinesCodec`/`BufReader::lines()` for leader TCP reads.
- [ ] Multi-byte UTF-8 split across reads is preserved.
- [ ] Newline-delimited framing unchanged.
- [ ] Add a regression test with split UTF-8.

## Design Impact

No change to TUI element design or composition. Only leader transport behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit test sending a multi-byte char split across two TCP reads.
- **Layer 2 — Event Handling:** Leader actor emits the correct parsed message event.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** A headless client sends Unicode through leader TCP and receives correct response.
- **Live tmux validation:** N/A (non-TUI path).

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
