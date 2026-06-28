# Initialize `tracing` subscriber in CLI and TUI binaries

**Status**: todo
**Milestone**: R5
**Category**: Observability
**Priority**: P0

**Depends on**: none
**Blocks**: replace-custom-telemetry-with-tracing-layer

## Description

`tracing` is declared as a workspace dependency and is used in a handful of `runie-core` modules, but neither `runie-cli` nor `runie-tui` initializes a `tracing-subscriber`. As a result, tracing events are silently dropped. Add `tracing-subscriber` to the workspace and initialize it in both binaries with an `EnvFilter` driven by `RUST_LOG`.

## Acceptance Criteria

- [ ] Add `tracing-subscriber` to workspace dependencies.
- [ ] Add it to `runie-cli` and `runie-tui` manifests.
- [ ] Initialize a subscriber in `crates/runie-cli/src/main.rs` at startup.
- [ ] Initialize a subscriber in `crates/runie-tui/src/main.rs` at startup.
- [ ] Default filter is `info` or respects `RUST_LOG`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `subscriber_rejects_invalid_filter` — an invalid `RUST_LOG` value is handled gracefully.

### Layer 2 — Event Handling
- [ ] `tracing_event_emitted_on_config_load` — a `ConfigLoaded` fact produces a matching `tracing` event in a test subscriber.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `Cargo.toml`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-cli/src/main.rs`
- `crates/runie-tui/src/main.rs`

## Notes

- Use `tracing_subscriber::fmt::init()` or a layered subscriber with `EnvFilter`.
- Keep the subscriber initialization simple; advanced telemetry layers belong in `replace-custom-telemetry-with-tracing-layer.md`.
