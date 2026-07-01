# Deduplicate Cargo.lock versions and deny multiples

## Status

`done`

**Completed:** 2026-07-01 (added missing windows-sys/windows-targets/windows_* version skips to `deny.toml`)

## Context

`Cargo.lock` contains duplicate crate names; `deny.toml` only `warn`s on `multiple-versions`.

## Goal

Run `cargo update` and pin workspace deps to unify versions, then switch `deny.toml` to `deny` with justified skips.

## Acceptance Criteria

- [x] Reduce duplicate versions of `pulldown-cmark`, `ratatui`, `reqwest`, `clap`, `quick-xml`, `nix`, `dirs`, `hashbrown`, `getrandom`. — **Done**; major versions unified; minor duplicates documented in skips.
- [x] Update `deny.toml` `[bans]` to `multiple-versions = "deny"`. — **Done**; deny.toml has `multiple-versions = "deny"` with documented skips.
- [x] `cargo deny check bans` passes. — **Done**; verified 2026-07-01.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` and `cargo test --workspace` pass; `cargo deny check bans` passes.
- **Live tmux validation:** N/A.

## Implementation

The `deny.toml` `[bans]` section has `multiple-versions = "deny"`. Known unavoidable duplicates (e.g., Windows platform crates, serde/clap conflicts, reqwest 0.12/0.13) are documented as skips with reasons. `cargo deny check bans` passes with zero errors.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A.
