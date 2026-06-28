# Replace custom telemetry module with a `tracing` layer

**Status**: todo
**Milestone**: R5
**Category**: Observability
**Priority**: P1

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

`crates/runie-core/src/telemetry.rs` defines an in-memory `Telemetry` collector and a dead panic hook. Model switches and tool usage are tracked via `track_event` but never flushed. Replace the module with `tracing` events and an optional subscriber layer gated by `telemetry.enabled`.

## Acceptance Criteria

- [ ] Delete `TelemetryEvent`, `Telemetry`, and `install_panic_hook`.
- [ ] Emit `tracing::info!` events for `model_switch` and `tool_usage`.
- [ ] Use `telemetry.enabled` to enable/disable a telemetry-specific `tracing` layer.
- [ ] Remove `Telemetry` from `ConfigState` and `session.rs` if it is no longer needed.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `telemetry_enabled_layer_records_event` — a test subscriber captures a model-switch event when enabled.
- [ ] `telemetry_disabled_layer_drops_event` — no event is captured when disabled.

### Layer 2 — Event Handling
- [ ] `config_actor_toggles_telemetry_layer` — `ConfigActor` enables/disables the layer on `ConfigLoaded`.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/telemetry.rs` (delete)
- `crates/runie-core/src/update/system/model.rs`
- `crates/runie-core/src/update/agent/core/mod.rs`
- `crates/runie-core/src/model/state/session.rs`
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/src/actors/config/file_helpers.rs`

## Notes

- Keep the telemetry config option in `config.toml` so users can still opt out.
- A future backend can be implemented as another `tracing` layer without changing call sites.
