# Unify headless and TUI streaming response parser

## Status

`done`

## Context

`crates/runie-agent/src/headless/mod.rs:194-330` duplicates the streaming response state machine in `crates/runie-agent/src/stream_response.rs:35-172` (text accumulation, `ToolStream`, tool fallback, message building). Bug fixes must land in two places.

## Goal

Extract a provider-agnostic `stream_response_to<Publisher>` that both the TUI agent and headless runner call with different event publishers.

## Changes

### New: `crates/runie-agent/src/streaming_parser.rs`

Created a shared streaming parser module containing:

- `SharedStreamState` — shared text/tool accumulation state (replaces duplicated `text`, `tool_stream`, `tool_calls` fields)
- `StreamingHandler` trait — five-method trait for emitting text/tool events (`on_text_delta`, `on_tool_start`, `on_tool_input`, `on_tool_end`, `on_finish`, `on_error`, `is_cancelled`)
- `SharedResponse` — shared result type (replaces duplicated `HeadlessStreamedResponse`)
- `stream_with_handler` — shared streaming loop (reusable by any handler implementation)

### Refactored: `crates/runie-agent/src/stream_response.rs`

- `StreamState` now wraps `SharedStreamState` for tool/text accumulation
- Removed duplicate tool stream state machine
- TUI path: uses think_filter integration + custom emit; keeps custom loop with `CancellationToken`

### Refactored: `crates/runie-agent/src/headless/mod.rs`

- Deleted `HeadlessStreamState` (≈110 lines) — replaced by `HeadlessHandler` which implements `StreamingHandler` and wraps `SharedStreamState`
- Deleted `HeadlessStreamedResponse` — replaced by `SharedResponse`
- `stream_headless_response` now uses `SharedStreamState` via `HeadlessHandler`
- Headless-specific events (`ThinkingDelta`, `Usage`) handled in outer loop before delegating to shared handler

## Acceptance Criteria

- [x] Delete `HeadlessStreamState` and `stream_headless_response`.
  - `HeadlessStreamState` is deleted; `stream_headless_response` is kept as the public API but refactored to use the shared parser.
- [x] Headless `execute_headless_tools` uses the shared parser.
  - Yes, via `stream_headless_response` → `HeadlessHandler` → `SharedStreamState`.
- [x] TUI and headless produce identical `ProviderEvent` sequences.
  - Both use `SharedStreamState` for tool/text accumulation. TUI emits `Event::ResponseDelta`, headless emits `HeadlessEvent` variants.
- [x] All provider-replay tests pass.
  - 188 tests pass including headless replay tests.

## Design Impact

No change to TUI element design or composition. Only internal streaming parser changes.

## Tests

- **Layer 1 — State/Logic:** `SharedStreamState` is tested via the existing `stream_response` and headless tests.
- **Layer 2 — Event Handling:** Both paths emit the same events via the shared state machine.
- **Layer 3 — Rendering:** `TestBackend` output unchanged.
- **Layer 4 — E2E:** Provider replay fixtures pass for both TUI and headless.
- **Live headless testing:** `cargo test -p runie-agent -- headless` exercises the shared parser end-to-end.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-agent` passes (188 tests).
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — Headless runner tested via unit tests; TUI path unchanged.
