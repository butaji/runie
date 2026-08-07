# P58 — Async replay/provider I/O

Status: complete (2026-08-08)

## Finding

The replay provider and replay HTTP actor exposed synchronous constructors
which read SSE fixture files with `std::fs` even though they are consumed by
async provider/test flows. This was a blocking I/O path hidden behind an
otherwise asynchronous transport boundary.

## Change

`ReplayProvider::from_sse` and `ReplayHttpActor::from_sse` now await
`tokio::fs::read_to_string`. Replay integration tests await construction and
continue to exercise the same provider → loop event sequence. No transport
wire shape or replay event ordering changed.

## Verification

- replay-provider integration suite passes
- `cargo fmt --all -- --check`
- `just ci`

Capture/comparison binaries retain synchronous file reads because they are
offline command-line tooling, not runtime actor/provider paths.
