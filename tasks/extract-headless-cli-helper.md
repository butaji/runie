# Extract headless CLI setup helper

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Three binary crates repeat the same headless-setup boilerplate: `runie-print/src/main.rs:26-59`, `runie-json/src/main.rs:84-145`, `runie-server/src/main.rs:37,201,216` all do `spawn_headless_runtime → provider(None,None) → PermissionGate::new(PermissionManager::default(), <sink>) → run_headless_turn(messages, provider, options)`. Any change to headless setup must be made 3×.

## Acceptance Criteria

- [ ] A single helper (e.g. `run_headless_cli(provider, prompt, opts)` or a `HeadlessCli` builder) lives in `runie-agent` (or `runie-core::headless_runtime`).
- [ ] `print`, `json`, `server` mains call the helper instead of inlining the wiring.
- [ ] The approval sink is the only caller-specific parameter.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `headless_cli_helper_builds_gate` — helper constructs a `PermissionGate` with the supplied sink.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_json_server_use_helper` — each binary still runs a one-shot turn against a mock provider.

## Files touched

- `crates/runie-agent/src/lib.rs` (or `crates/runie-core/src/headless_runtime.rs`)
- `crates/runie-print/src/main.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`

## Notes

Keep the helper signature minimal — one `ApprovalSink` parameter is the only real variance. `consolidate-binary-setup` already moved some setup into `runie-core::headless_runtime`; extend that rather than creating a new module.
