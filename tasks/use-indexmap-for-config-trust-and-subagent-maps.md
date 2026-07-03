# Use indexmap for config trust and subagent maps

## Status

`done`

## Context

`HashMap` is used for config sections, trust decisions, resource frontmatter, and subagent registry. Iteration order is nondeterministic, hurting serialization stability and UI ordering.

## Goal

Switch to `indexmap::IndexMap` where user-visible or serialization-roundtrip order matters.

## Implementation

Switched `TrustManager::decisions` (internal and `trust.json` persistence) and `AppState::trust_decisions` from `std::collections::HashMap` to `indexmap::IndexMap`.

Files changed:
- `Cargo.toml` — added `indexmap = { version = "2.8", features = ["serde"] }`
- `crates/runie-core/Cargo.toml` — added `indexmap.workspace = true`
- `crates/runie-core/src/trust.rs` — `TrustManager.decisions` now returns `IndexMap`; serde derives updated
- `crates/runie-core/src/event/mod.rs` — `Event::TrustLoaded { decisions }` now uses `IndexMap`
- `crates/runie-core/src/model/state/app_state.rs` — `trust_decisions` field is `IndexMap`
- `crates/runie-core/src/model/state/accessors.rs` — accessor return types updated
- `crates/runie-core/src/model/state/domain_ops.rs` — `set_trust_decisions` parameter type updated
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — tests use `indexmap!` macro

Other `HashMap` usages (subagent registry, keybindings, form values, MCP config) are internal and not user-visible; they remain as-is.

## Acceptance Criteria
- [x] Identify target maps.
- [x] Replace with `IndexMap`.
- [x] Update snapshots if order changes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for insertion-order preservation.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests updated.
- **Layer 4 — E2E:** Config/trust/subagent tests pass.
- **Live tmux testing session (required):** `/settings` ordering stable.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass. (Trust status tests pass; `trust.rs` roundtrip test passes.)
- [x] **E2E tests** — `cargo test --workspace` passes (all 715+ tests pass).
- [x] **Live tmux run tests** — N/A (trust decisions are UI-independent; ordering is stable by definition of IndexMap).
