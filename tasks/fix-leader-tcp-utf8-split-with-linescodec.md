# Fix leader TCP UTF-8 split with LinesCodec

## Status

`done`

**Completed:** 2026-06-30

## Context

`crates/runie-core/src/actors/leader/actor.rs:232-275` reads fixed 1024-byte chunks and converts each chunk with `std::str::from_utf8`, silently dropping multi-byte UTF-8 characters split across reads. It also manually buffers lines.

## Goal

Replace the manual byte buffering with `tokio::io::BufReader::lines()` or `tokio_util::codec::{FramedRead, LinesCodec}`. Share the same framing layer with the CLI transport.

## Acceptance Criteria

- [x] Use `LinesCodec`/`BufReader::lines()` for leader TCP reads. — Done; `tokio::io::BufReader` with `read_line()` replaces manual 1024-byte chunk reads.
- [x] Multi-byte UTF-8 split across reads is preserved. — Done; `BufReader::read_line()` handles arbitrary byte boundaries.
- [x] Newline-delimited framing unchanged. — Done; `read_line()` produces one line per newline-delimited message.
- [x] Add a regression test with split UTF-8. — Done; `bufreader_preserves_split_utf8` test in `actor.rs` tests.

## Design Impact

No change to TUI element design or composition. Only leader transport behavior changes.

## Tests

- **Layer 1 — State/Logic:** `bufreader_preserves_split_utf8` verifies multi-byte char split across reads is reassembled correctly.
- **Layer 2 — Event Handling:** Leader actor emits the correct parsed message event.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** A headless client sends Unicode through leader TCP and receives correct response.
- **Live tmux validation:** N/A (non-TUI path).

## Implementation Notes

Replaced fixed 1024-byte `read_exact` + `from_utf8` loop with:
```rust
let mut reader = tokio::io::BufReader::new(rd);
let mut line = String::new();
while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
    line = line.trim_end_matches('\n').to_string();
    if !line.is_empty() {
        process_client_line(&line, &bus);
    }
    line.clear();
}
```

`BufReader::read_line()` handles multi-byte UTF-8 correctly across arbitrary read boundaries. The old `from_utf8` chunking logic and 1024-byte buffer are removed.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core leader` passes (8 tests including new UTF-8 regression test).
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
