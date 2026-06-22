# Fix runie-tui dead_code warnings

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`cargo check --workspace` currently emits 4 `dead_code` warnings, all in `runie-tui`:

```
warning: function `load_theme_raw_async` is never used
warning: function `load_theme_async` is never used
warning: function `load_theme_with_caps_async` is never used
warning: fields `0`, `1`, `2`, `3`, and `4` are never read
```

The three async theme loaders in `crates/runie-tui/src/theme/loader.rs:31,43,61` were a planned offload-the-runtime variant that never got adopted; the synchronous `load_theme_raw` / `load_theme` / `load_theme_with_caps` paths are the only ones called. The `ActorHandles` tuple struct at `crates/runie-tui/src/main.rs:92-98` is bound to `let _actors = bootstrap.2;` and never read — only the third tuple element (`ProviderActorHandle`) is destructured into a used variable.

These are pure deletions with no test fallout.

## Acceptance Criteria

- [ ] `load_theme_raw_async`, `load_theme_async`, `load_theme_with_caps_async` deleted from `crates/runie-tui/src/theme/loader.rs`.
- [ ] `ActorHandles` struct deleted from `crates/runie-tui/src/main.rs`.
- [ ] `bootstrap_app` return type updated to drop `ActorHandles`; `let _actors = bootstrap.2;` replaced with destructuring of the provider handle.
- [ ] `cargo check --workspace` succeeds with zero new warnings (the 4 listed above are gone; pre-existing warnings, if any, are out of scope).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure deletion, no logic.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_runie_tui_builds_warning_free` — `cargo check -p runie-tui` produces zero warnings (other than any pre-existing ones unrelated to dead code).

## Files touched

- `crates/runie-tui/src/theme/loader.rs`
- `crates/runie-tui/src/main.rs`

## Notes

Trivial. First cleanup to land because it removes compiler warnings visible in every build. If a future need arises for async theme loading, reintroduce it as a real call site, not a dangling helper.
