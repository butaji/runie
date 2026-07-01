# Remove blanket clippy allow in turn projections

## Status

`done`

## Context

`crates/runie-core/src/model/state/turn_projections.rs:1` had `#![allow(clippy::all)]`, hiding real quality issues in production code.

## Changes

- Removed `#![allow(clippy::all)]` from `turn_projections.rs`
- No new clippy warnings were introduced

## Acceptance Criteria
- [x] Remove `#![allow(clippy::all)]`.
- [x] Fix or individually allow remaining lints (none found).
- [x] `cargo clippy --workspace` passes.

## Tests

- **Layer 4 — E2E:** Clippy clean.
