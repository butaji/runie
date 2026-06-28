# Narrow `runie-core` public API

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor, collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/lib.rs` currently re-exports roughly forty modules, exposing many internal utilities. Some of those utilities are legitimately used by downstream crates (`runie-tui`, `runie-provider`, `runie-cli`, and proc-macros in `runie-macros`), so narrowing visibility blindly will break the workspace. This task first does a workspace-wide usage audit, then either (a) keeps a module public if it has external consumers, (b) moves it to a new lightweight utility crate if it is shared but not core-domain, or (c) narrows it to `pub(crate)` if it is purely internal.

Current state as of this review:

- `lib.rs` re-exports ~43 `pub mod` declarations plus many `pub use` items.
- Rough workspace usage audit:
  - `display_width` — used by `runie-tui` → candidate for `runie-util`.
  - `path` — used by `runie-agent` → candidate for `runie-util`.
  - `sanitize` — used by `runie-agent`, `runie-provider` → candidate for `runie-util`.
  - `labels` — used by `runie-tui` production code (`status_bar.rs`) and tests → candidate for `runie-util`.
  - `display_width`, `path`, `sanitize` — used by downstream crates → candidates for `runie-util`.
  - `build_lint`, `declarative`, `dry_run`, `edit_preview`, `file_refs`, `fuzzy`, `glob`, `input_history`, `notification`, `scoped_model`, `streaming_buffer`, `telemetry` — internal-only or lightly used → candidates for `pub(crate)`.
  - `actors`, `event`, `model`, `tool`, `view`, `provider`, `config`, `permissions`, `message` — heavily used → must stay public.
- Many public actor-handle types (`ActorHandles`, `Ractor*Handle`, etc.) are still being migrated to `ractor`. Narrowing visibility before the actor migration finishes will create churn.

## Acceptance Criteria

- [ ] Produce an explicit "keep public / move to util / pub(crate)" table and record the rationale for each decision.
- [ ] Create `crates/runie-util/` (or a similarly named lightweight utility crate) and move `display_width`, `labels`, `path`, and `sanitize` there.
- [ ] Keep modules public that are used by `runie-tui`, `runie-provider`, `runie-cli`, or `runie-macros`.
- [ ] Keep `runie-core::config` public because `runie-provider` re-exports `Config`, `ModelProvider`, and `ModelsSection` from it.
- [ ] Convert modules that have no external consumers to `pub(crate)`.
- [ ] Keep the documented public surface exported and stable: `AppState`, `Event`, actor handles, provider trait, session types, and commands registry.
- [ ] Update downstream crates so that `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `documented_exports_are_present` — verifies that the documented public items remain reachable through `runie-core` from an external crate context.
- [ ] `workspace_usage_audit_documented` — asserts that every `runie-core` module import from another workspace crate is either kept public or moved to a utility crate with a recorded rationale.

### Layer 2 — Event Handling
- [ ] N/A — this task changes module visibility, not event dispatch or input handling.

### Layer 3 — Rendering
- [ ] N/A — this task changes crate-level API boundaries, not widget rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `public_api_does_not_expose_internals` — compiles a small external-crate smoke test that attempts to import former internal utilities and confirms the build fails or resolves from the new utility crate, while documented public items resolve successfully.

## Files touched

- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/*/mod.rs` (as needed to narrow re-exports)
- New `crates/runie-util/Cargo.toml` and `src/lib.rs`
- Downstream call sites in `crates/runie-agent/`, `crates/runie-cli/`, `crates/runie-tui/`, and other workspace crates that currently import internal `runie-core` items.

## Notes

- Prefer `pub(crate)` over private modules so that internal tests and sibling modules in `runie-core` can still access helpers without expanding the public API.
- Defer aggressive narrowing until after `migrate-production-actors-to-ractor` and `collapse-actor-handles-to-typed-map` so that handle types stop moving.
- If a downstream crate legitimately needs a helper, move it to a dedicated utility crate rather than leaving it in `runie-core`. Do not recreate `runie-io`/`runie-domain`; those were deleted as empty facades.
- Rejected alternative: using a `#[doc(hidden)]` attribute on internal items. Hiding items does not provide the same compile-time API contract as `pub(crate)` and still permits accidental public dependence.
- Out of scope: changing function bodies, renaming items, or modifying the provider trait surface. Visibility changes only.
