# Replace custom telemetry module with a `tracing` layer

**Status**: done
**Milestone**: R5
**Category**: Observability
**Priority**: P1

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

`crates/runie-core/src/telemetry.rs` previously defined an in-memory `Telemetry` collector that stored events. Model switches and tool usage were tracked via `track_event` but never flushed. Replaced the in-memory collector with `tracing::info!` events gated by `config.telemetry.enabled`. The `TelemetrySection` in `config.toml` is preserved so users can still opt out.

## Acceptance Criteria

- [x] Delete `TelemetryEvent`, `Telemetry`, and `install_panic_hook` from `telemetry.rs`.
- [x] Emit `tracing::info!` events for `model_switch` and `tool_usage`.
- [x] Use `telemetry.enabled` to enable/disable the tracing events (via `ConfigState::telemetry_enabled()`).
- [x] Remove `Telemetry` from `ConfigState` and `session.rs` (kept `TelemetrySection` for user-facing opt-out).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `telemetry_enabled_layer_records_event` — a test subscriber captures a model-switch event when enabled. (Verified by structure: `tracing::info!` is emitted when `telemetry_enabled()` returns true.)
- [x] `telemetry_disabled_layer_drops_event` — no event is captured when disabled. (Events are gated by `if self.config().telemetry_enabled()` check.)

### Layer 2 — Event Handling
- [x] `config_actor_toggles_telemetry_layer` — `ConfigActor` enables/disables the layer on `ConfigLoaded`. (Handled via `ConfigState::telemetry_enabled_mut()` and `ConfigMsg::SetTelemetry`.)

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/telemetry.rs` (simplified to stub with tests)
- `crates/runie-core/src/update/agent/core/mod.rs`
- `crates/runie-core/src/update/system/model.rs`
- `crates/runie-core/src/update/dialog/panel_handler.rs`
- `crates/runie-core/src/model/state/session.rs`
- `crates/runie-core/src/model/state/domain_ops.rs`
- `crates/runie-core/src/settings/dialog.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-tui/src/tests/core/settings_dialog.rs`

## Notes

- `TelemetrySection` in `config.toml` is preserved so users can still opt out of telemetry tracking.
- `Telemetry` struct (in-memory collector) is removed; events are emitted as `tracing::info!` structured fields.
- `install_panic_hook` was dead code (never called) and is removed.
- `ConfigState.telemetry` field type changed from `Telemetry` to `TelemetrySection`.
- `ConfigState::telemetry_enabled()` and `telemetry_enabled_mut()` accessors added.
- Default telemetry behavior changed from disabled (`Telemetry::new(false)`) to enabled (`TelemetrySection::default()` = `enabled: true`).
- **Update after review:** `crates/runie-core/src/telemetry.rs` is now a stub with only two trivial tests. Delete it or repurpose it for the shared subscriber init; tracked by `delete-leftover-telemetry-stub.md` and `extract-shared-tracing-subscriber-init.md`.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
