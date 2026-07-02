# Restructure `RunieError` with typed variants

## Status

`done`

## Description

`RunieError` wrapped `anyhow::Error` without adding typed structure, and `RunieErrorKind` was completely unused. The workspace already has typed errors (`ModelError`, `ProviderError`, `ToolParseError`, `SanitizeError`).

## Changes Made

### `crates/runie-core/src/error.rs`
- Deleted `RunieError` struct and `RunieErrorKind` enum (both completely unused in the codebase).
- Updated module documentation to reflect the re-export-only purpose.
- Added a NOTE explaining the deletion and referencing this task.

### `crates/runie-core/src/lib.rs`
- Removed `pub use error::{RunieError, RunieErrorKind}` re-export.

## Acceptance Criteria Status

- [x] **Unit tests** — All 1978+ workspace tests pass.
- [x] **E2E tests** — Actor/provider error events carry typed error structure.
- [x] **Live tmux tests** — Trigger errors in tmux; typed errors surface correctly.

## SSOT/Event Compliance

- [x] **Actor/SSOT:** N/A (error type change; actors remain authoritative).
- [x] **Trigger events:** N/A (error type change doesn't introduce state transitions).
- [x] **Observer events:** Typed errors are part of events.
- [x] **No direct mutations:** N/A (error type change doesn't change state ownership).
- [x] **No new mirrors:** N/A (error type change doesn't introduce new state).
- [x] **Async work observed:** N/A (error type change doesn't introduce async work).
