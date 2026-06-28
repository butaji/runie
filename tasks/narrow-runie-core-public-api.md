# Narrow `runie-core` public API

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/lib.rs` currently re-exports roughly forty modules, exposing many internal utilities that downstream crates do not need and that are not part of the documented public surface. This makes API evolution risky and hides the true boundaries of the crate. This task audits every `pub mod` and `pub use` declaration in `runie-core`, converts implementation-only modules to `pub(crate)`, and keeps only the documented public surface (`AppState`, `Event`, actor handles, the provider trait, session types, and the commands registry) visible to other crates.

## Acceptance Criteria

- [ ] Audit all `pub mod` and `pub use` declarations in `crates/runie-core/src/lib.rs` and nested module roots.
- [ ] Convert implementation-only modules (`glob`, `fuzzy`, `sanitize`, `layout`, `labels`, `path`, `display_width`, `edit_preview`, and similar helpers) to `pub(crate)` or move them behind feature gates.
- [ ] Keep the documented public surface exported and stable: `AppState`, `Event`, actor handles, provider trait, session types, and commands registry.
- [ ] Update downstream crates so that `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `documented_exports_are_present` — verifies that the documented public items (`AppState`, `Event`, actor handle types, provider trait, session types, and commands registry) remain reachable through `runie-core` from an external crate context.

### Layer 2 — Event Handling
- [ ] N/A — this task changes module visibility, not event dispatch or input handling.

### Layer 3 — Rendering
- [ ] N/A — this task changes crate-level API boundaries, not widget rendering.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `public_api_does_not_expose_internals` — compiles a small external-crate smoke test that attempts to import former internal utilities and confirms the build fails, while documented public items resolve successfully.

## Files touched

- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/*/mod.rs` (as needed to narrow re-exports)
- Downstream call sites in `crates/runie-agent/`, `crates/runie-cli/`, `crates/runie-tui/`, and other workspace crates that currently import internal `runie-core` items.

## Notes

- Prefer `pub(crate)` over private modules so that internal tests and sibling modules in `runie-core` can still access helpers without expanding the public API.
- If a downstream crate legitimately needs a helper, consider promoting it to a dedicated utility crate (e.g., `runie-io` or a new `runie-text`) rather than leaving it in `runie-core`.
- Rejected alternative: using a `#[doc(hidden)]` attribute on internal items. Hiding items does not provide the same compile-time API contract as `pub(crate)` and still permits accidental public dependence.
- Out of scope: changing function bodies, renaming items, or modifying the provider trait surface. Visibility changes only.
