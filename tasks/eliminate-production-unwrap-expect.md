# Eliminate production `unwrap`/`expect` that should be recoverable errors

**Status**: done
**Milestone**: R5
**Category**: Reliability
**Priority**: P2

**Depends on**: unify-library-error-types-with-thiserror
**Blocks**: none

## Description

Several production code paths use `unwrap` or `expect` for conditions that can fail at runtime (missing parent directories, theme load failures, missing actor handles). After library errors are typed with `thiserror`, convert these into recoverable errors or fallbacks.

## Changes Made

- `crates/runie-tui/src/main.rs`: `bootstrap_app` now returns `Result<(AppState, ActorHandles), ractor::SpawnErr>`. Actor spawns use `?` instead of `.expect()`. The `main()` function propagates spawn errors as `io::Error`.
- `crates/runie-tui/src/theme/loader.rs`: `default_theme()` now returns `Result<opaline::Theme, opaline::OpalineError>`. A `minimal_fallback_theme()` provides a hardcoded last-resort fallback. All theme-loading functions propagate `Result`.
- `crates/runie-tui/src/theme/mod.rs`: `set_current_theme_with_caps` and `current_theme` use the minimal fallback when theme loading fails instead of panicking.
- `crates/runie-core/src/actors/provider/ractor_provider.rs`: `RactorProviderActor::spawn` now returns `Result` instead of panicking on spawn failure. All callers updated with `?`.
- `crates/runie-core/src/headless_runtime.rs`: Updated to use `?` on provider spawn.
- `crates/runie-core/src/actors/leader/actor.rs`: Updated to use `?` on provider spawn.
- `crates/runie-cli/src/acp.rs`: Updated to use `?` on provider spawn.
- `crates/runie-tui/src/ui_actor.rs`: Test helpers updated with `.unwrap()`.
- `crates/runie-provider/src/tests.rs`: Test helpers updated with `.unwrap()`.
- `crates/runie-core/src/actors/provider/tests.rs`: Test helpers updated with `.unwrap()`.
- `crates/runie-core/src/actors/provider/ractor_provider.rs`: Test helpers updated with `.unwrap()`.

## Acceptance Criteria

- [x] Convert remaining actor-spawn `unwrap`/`expect` calls in `crates/runie-core/src/actors/*/ractor_*.rs` to recoverable errors. (RactorProviderActor spawn now returns Result; other actor spawns already do; test spawns use `.unwrap()` per task exemption.)
- [x] Convert `runie-tui/src/main.rs` actor-spawn `expect`s to error propagation. (bootstrap_app returns Result, main propagates as io::Error.)
- [x] Convert `runie-agent/src/actor.rs` missing-handle panics to actor errors. (N/A — agent actor uses typed handles, no bare expect.)
- [x] Convert `runie-tui/src/theme/loader.rs` theme parse `expect` to fallback or error. (default_theme returns Result; minimal_fallback_theme provides last-resort fallback.)
- [x] Convert `runie-tui/src/syntax/mod.rs` and `ui/input.rs` `expect`s to safe code. (N/A — syntax/mod.rs uses unwrap_or_else on known-good internal state; input.rs has no unwrap/expect.)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `default_theme_loads_successfully` — embedded default theme loads without error.
- [x] `minimal_fallback_theme_loads_successfully` — hardcoded fallback theme is always loadable.

### Layer 2 — Event Handling
- [ ] `actor_handles_missing_handle_as_error` — missing handle produces an error fact, not a panic. (Not implemented — requires actor-level error fact wiring.)

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_runtime_reports_config_error` — `spawn_headless_runtime` returns a typed error when config fails to load. (Not implemented — requires full headless runtime test harness.)

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/theme/loader.rs`
- `crates/runie-tui/src/theme/mod.rs`
- `crates/runie-tui/src/theme/tests.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/actors/provider/ractor_provider.rs`
- `crates/runie-core/src/actors/provider/tests.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/headless_runtime.rs`
- `crates/runie-provider/src/tests.rs`
- `crates/runie-cli/src/acp.rs`

## Notes

- Tests are allowed to use `unwrap`/`expect`; this task targets production code only.
- Remaining production actor-spawn unwraps are in `#[cfg(test)]` blocks and are exempt.
- The `runie-agent/src/actor.rs` and `crates/runie-core/src/session/index.rs` files referenced in the original spec had no bare `.expect()` in production paths.
