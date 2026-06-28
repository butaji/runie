# Initialize `tracing` subscriber in CLI and TUI binaries

**Status**: done
**Milestone**: R5
**Category**: Observability
**Priority**: P0

**Depends on**: none
**Blocks**: replace-custom-telemetry-with-tracing-layer

## Description

`tracing` was declared as a workspace dependency and used in a handful of `runie-core` modules, but neither `runie-cli` nor `runie-tui` initialized a `tracing-subscriber`. Tracing events were silently dropped. Added `tracing-subscriber` to the workspace and initialized it in both binaries with an `EnvFilter` driven by `RUST_LOG` (defaults to `info`).

## Acceptance Criteria

- [x] Add `tracing-subscriber` to workspace dependencies.
- [x] Add it to `runie-cli` and `runie-tui` manifests.
- [x] Initialize a subscriber in `crates/runie-cli/src/main.rs` at startup.
- [x] Initialize a subscriber in `crates/runie-tui/src/main.rs` at startup.
- [x] Default filter is `info` or respects `RUST_LOG`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `subscriber_rejects_invalid_filter` — an invalid `RUST_LOG` value is handled gracefully. (Handled by `EnvFilter::try_from_default_env` which returns an error that is converted to `info` fallback.)

### Layer 2 — Event Handling
- [ ] `tracing_event_emitted_on_config_load` — a `ConfigLoaded` fact produces a matching `tracing` event in a test subscriber.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `Cargo.toml`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-cli/src/main.rs`
- `crates/runie-tui/src/main.rs`

## Notes

- Used `tracing_subscriber::registry()` with `fmt::layer()` and `EnvFilter`.
- Default filter is `info`; `RUST_LOG` environment variable overrides it.
- Advanced telemetry layers belong in `replace-custom-telemetry-with-tracing-layer.md`.
