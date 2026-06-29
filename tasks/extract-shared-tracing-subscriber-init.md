# Extract shared tracing subscriber init

**Status**: todo
**Milestone**: R6
**Category": Observability
**Priority": P2

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

`runie-tui/src/main.rs` and `runie-cli/src/main.rs` contain identical `EnvFilter` + `fmt::layer` subscriber setup. Extract a single `runie_core::telemetry::init()` helper and call it from both binaries.

## Acceptance Criteria

- [ ] Add `runie_core::telemetry::init()` that builds the subscriber.
- [ ] Call it from `runie-tui/src/main.rs` and `runie-cli/src/main.rs`.
- [ ] Preserve env-filter behavior and default filter.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `subscriber_init_is_idempotent` — calling init twice does not panic.

### Layer 2 — Event Handling
- [ ] `telemetry_event_emitted_after_init` — a test subscriber captures an event.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/telemetry.rs` (currently a stub; repurpose)
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/main.rs`

## Notes

- This can be combined with deleting the leftover telemetry stub if the stub has no other purpose.
