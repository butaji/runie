# Unify provider credential resolution with `dotenvy`

**Status**: done
**Milestone**: R2
**Category**: Provider / Configuration
**Priority**: P1
**Note**: runie-provider/src/config/mod.rs still manually re-parses .env after dotenvy::dotenv.

**Depends on**: route-cli-config-through-configactor
**Blocks**: none

## Description

Provider credential resolution was duplicated between `crates/runie-provider/src/config/mod.rs` (custom `.env` parser) and the env-var fallback in `runie-core`. Added `dotenvy` to load `.env` centrally and replaced the custom parser.

## Changes

- Added `dotenvy = "0.15"` to workspace dependencies and relevant crates.
- Replaced custom `.env` parser in `runie-provider/src/config/mod.rs` with `dotenvy::dotenv()`.
- Env var fallback still takes priority over `.env` file.

## Acceptance Criteria

- [x] Add `dotenvy` to workspace dependencies.
- [x] `runie-provider` uses `dotenvy` instead of custom parser.
- [x] Env vars still take priority over `.env` file.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `resolve_env_takes_priority` — env vars override config.
- [x] `resolve_config_fallback` — config used when no env var.
- [x] `empty_config_returns_none` — handles missing values gracefully.

## Files touched

- `Cargo.toml` (added `dotenvy` workspace dep)
- `crates/runie-core/Cargo.toml`
- `crates/runie-provider/Cargo.toml`
- `crates/runie-provider/src/config/mod.rs` (replaced custom parser with dotenvy)
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
