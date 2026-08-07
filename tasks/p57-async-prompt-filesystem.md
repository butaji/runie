# P57 — Async prompt filesystem boundary

Status: complete (2026-08-08)

## Finding

`PromptActor` is the SSOT owner for prompt state, but its file-search command
called `std::fs::read_dir` and `std::fs::read_to_string` inside the actor
worker. That made the event loop synchronous during directory enumeration or
file preview, violating the reactive/non-blocking TUI contract.

## Change

The actor now awaits `PromptWidget::open_file_search_async`, which uses
Tokio's filesystem APIs. The reducer remains the only place that applies the
resulting candidate/preview facts and publishes the immutable prompt
snapshot; rendering and key handling remain non-blocking.

## Verification

- async prompt actor tests pass, including the filesystem transition
- `cargo fmt --all -- --check`
- `just ci`

The synchronous widget helper remains only for isolated legacy unit fixtures;
the production actor path uses the async method.
