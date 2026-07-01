# Deduplicate Cargo.lock versions and deny multiples

## Status

`todo`

## Context

`Cargo.lock` contains 53 duplicate crate names; `deny.toml` only `warn`s on `multiple-versions`.

## Goal

Run `cargo update` and pin workspace deps to unify versions, then switch `deny.toml` to `deny` with justified skips.

## Acceptance Criteria
- [ ] Reduce duplicate versions of `pulldown-cmark`, `ratatui`, `reqwest`, `clap`, `quick-xml`, `nix`, `dirs`, `hashbrown`, `getrandom`.
- [ ] Update `deny.toml` `[bans]` to `multiple-versions = "deny"`.
- [ ] `cargo deny check bans` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` and `cargo test --workspace` pass; `cargo deny check bans` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
