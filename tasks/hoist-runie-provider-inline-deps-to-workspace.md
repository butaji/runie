# Hoist runie-provider inline deps to workspace

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-provider/Cargo.toml` pins `async-stream`, `reqwest-eventsource`, and `wiremock` inline.

## Goal

Move them to `[workspace.dependencies]` (dev-only for `wiremock`) and use `workspace = true`.

## Acceptance Criteria
- [x] Add `async-stream`, `reqwest-eventsource`, and `wiremock` to workspace deps. — Done; all three defined in `Cargo.toml`
- [x] Use `workspace = true` in `runie-provider/Cargo.toml`. — Done; all three use `workspace = true`
- [x] `cargo check -p runie-provider` passes. — Verified

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test -p runie-provider` passes (86 tests).
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-provider` passes.
- [x] **E2E tests** — `cargo test --workspace` passes.

## Implementation Verification

```
$ grep -E "async-stream|reqwest-eventsource|wiremock" crates/runie-provider/Cargo.toml
async-stream.workspace = true
reqwest-eventsource.workspace = true
wiremock.workspace = true

$ grep -E "async-stream|reqwest-eventsource|wiremock" Cargo.toml
async-stream = "0.3"
reqwest-eventsource = "0.6"
wiremock = "0.6"
```
