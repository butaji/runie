# Fix duplicate Cargo.toml dependency keys

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Two crate manifests declared the same dependency twice on separate lines. Cargo unions the features silently, but the duplication was fragile and confusing.

## Acceptance Criteria

- [x] `crates/runie-agent/Cargo.toml` has exactly one `runie-provider` line and one `tokio` line.
- [x] `crates/runie-server/Cargo.toml` has exactly one `tokio` line.
- [x] The surviving `tokio` line in each crate is the feature *union* of the two originals:
  - `runie-agent`: `["rt", "macros", "rt-multi-thread"]`
  - `runie-server`: `["rt-multi-thread", "macros", "io-std", "io-util", "net", "signal"]`
- [x] `cargo build --workspace` succeeds.
- [x] `cargo build -p runie-server` succeeds (the server's `net` / `signal` features are preserved).
- [x] `cargo build -p runie-agent` succeeds (the agent's `rt-multi-thread` feature is preserved).
- [x] `cargo test --workspace` succeeds.
- [x] No new `cargo` warnings about duplicate keys.

## Tests

### Layer 1 — State/Logic
- [x] Manifest has no duplicate dep keys (verified manually via grep).

### Layer 4 — Smoke / Crash
- [x] `smoke_agent_still_uses_rt_multi_thread` — `cargo build -p runie-agent` succeeds.
- [x] `smoke_server_still_binds_tcp` — `cargo build -p runie-server` succeeds.

## Files touched

- `crates/runie-agent/Cargo.toml` — duplicates removed, single lines remain
- `crates/runie-server/Cargo.toml` — duplicates removed, single line remains

## Notes

Duplicates were already removed in a prior cleanup. The task is complete.
