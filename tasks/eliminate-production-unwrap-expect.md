# Eliminate production `unwrap`/`expect` that should be recoverable errors

**Status**: todo
**Milestone**: R5
**Category**: Reliability
**Priority**: P2

**Depends on**: unify-library-error-types-with-thiserror
**Blocks**: none

## Description

Several production code paths use `unwrap` or `expect` for conditions that can fail at runtime (missing parent directories, theme load failures, missing actor handles). After library errors are typed with `thiserror`, convert these into recoverable errors or fallbacks.

## Acceptance Criteria

- [ ] Convert `runie-provider/src/lib.rs:287` `.expect("config must load")` to `Result`.
- [ ] Convert `runie-tui/src/main.rs` actor-spawn `expect`s to error propagation.
- [ ] Convert `runie-core/src/session/index.rs:54` `path.parent().unwrap()` to a safe error.
- [ ] Convert `runie-tui/src/theme/loader.rs:12` theme parse `expect` to fallback or error.
- [ ] Convert `runie-tui/src/syntax/mod.rs` and `ui/input.rs` `expect`s to safe code.
- [ ] Convert `runie-agent/src/actor.rs` missing-handle panics to actor errors.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `theme_fallback_on_invalid_embedded_theme` — invalid embedded theme returns a default instead of panicking.

### Layer 2 — Event Handling
- [ ] `actor_handles_missing_handle_as_error` — missing handle produces an error fact, not a panic.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_runtime_reports_config_error` — `spawn_headless_runtime` returns a typed error when config fails to load.

## Files touched

- `crates/runie-provider/src/lib.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-core/src/session/index.rs`
- `crates/runie-tui/src/theme/loader.rs`
- `crates/runie-tui/src/syntax/mod.rs`
- `crates/runie-tui/src/ui/input.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- Tests are allowed to use `unwrap`/`expect`; this task targets production code only.
- This task should be done after `unify-library-error-types-with-thiserror.md` so errors have a natural home.
