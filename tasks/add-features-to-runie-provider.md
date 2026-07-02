# Add features to `runie-provider`

## Status

`done`

## Description

`runie-provider` now supports feature flags so consumers can compile only what they need.

## Implementation

Added feature flags to `Cargo.toml`:
- `default = ["openai", "mock"]` — both providers enabled by default
- `openai` — OpenAI-compatible provider (requires `reqwest`, `async-stream`)
- `mock` — Mock provider for testing (no additional deps)

The `openai` and `mock` modules are gated with `#[cfg(feature = "...")]`.

The factory functions handle feature combinations gracefully:
- When `mock` is disabled and `key == "mock"`, returns `UnknownProvider`
- When `openai` is disabled, non-mock providers return `UnknownProvider`
- Tests compile correctly with all feature combinations

## Acceptance criteria

- [x] **Feature matrix compiles** — `--no-default-features`, `--features openai`, `--features mock`, `--features openai,mock`, and default all compile.
- [x] **Default includes needed providers** — Default build includes both `openai` and `mock`.
- [x] **`cargo test -p runie-provider` passes** — 124 tests pass.
- [x] **`cargo check --workspace` succeeds** — All crates compile with default features.

## Tests

### Unit tests
- Feature combinations compile without warnings.
- Mock provider works with `mock` feature enabled.
- OpenAI provider works with `openai` feature enabled.

### E2E tests
- Default feature replay passes (4 minimax + 4 openai replay tests).

### Live tmux tests
- N/A (build-only change).

## Files touched

- `crates/runie-provider/Cargo.toml` — Added feature flags
- `crates/runie-provider/src/lib.rs` — Gated modules and factory functions

## Notes

- Both features are enabled by default for backward compatibility
- Feature flags allow minimal builds for specialized use cases (e.g., headless-only with no mock)
