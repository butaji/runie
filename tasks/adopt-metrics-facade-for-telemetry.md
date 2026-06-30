# Adopt metrics facade for telemetry

## Status

`todo`

## Context

`crates/runie-core/src/update/system/model.rs:33`, `update/agent/core/mod.rs:90`, and `config/mod.rs:88-96` emit ad-hoc `tracing::info!` calls gated by `telemetry_enabled`. There is no counter/histogram facade, exporter, or consistent labels.

## Goal

Adopt the `metrics` crate facade (`counter!`, `histogram!`, `gauge!`) and optionally `metrics-exporter-prometheus` behind a feature flag. Convert `tool_usage` and `model_switch` to labeled counters.

## Acceptance Criteria

- [ ] Add `metrics` to workspace deps.
- [ ] Replace ad-hoc `tracing::info!` telemetry calls with `counter!`/`histogram!`.
- [ ] Wire a no-op recorder when telemetry is disabled.
- [ ] Optional: add Prometheus exporter behind feature flag.
- [ ] `telemetry_enabled` config flag still gates recording.

## Design Impact

No change to TUI element design or composition. Only telemetry emission behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests verify counter increments with labels.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Telemetry disabled by default; enabled path records metrics.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
