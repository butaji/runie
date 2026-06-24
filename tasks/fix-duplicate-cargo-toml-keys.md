# Fix duplicate Cargo.toml dependency keys

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Two crate manifests declare the same dependency twice on separate lines. Cargo unions the features silently and uses the last declaration's metadata, so it builds — but the duplication is fragile (the second `tokio` line in `runie-server` drops `net`, `signal`, `io-std`, `io-util` from the declared set, which only works because the first line re-adds them) and confuses readers / tooling.

| Crate | Line | Entry |
|-------|------|-------|
| `crates/runie-agent/Cargo.toml` | 12 | `runie-provider.workspace = true` |
| `crates/runie-agent/Cargo.toml` | 22 | `runie-provider.workspace = true` (duplicate) |
| `crates/runie-agent/Cargo.toml` | 18 | `tokio = { workspace = true, features = ["rt", "macros", "rt-multi-thread"] }` |
| `crates/runie-agent/Cargo.toml` | 23 | `tokio = { workspace = true, features = ["rt", "macros"] }` (duplicate, drops `rt-multi-thread`) |
| `crates/runie-server/Cargo.toml` | 17 | `tokio = { workspace = true, features = ["rt-multi-thread", "macros", "io-std", "io-util", "net", "signal"] }` |
| `crates/runie-server/Cargo.toml` | 20 | `tokio = { workspace = true, features = ["rt", "macros"] }` (duplicate, drops 4 features) |

## Acceptance Criteria

- [ ] `crates/runie-agent/Cargo.toml` has exactly one `runie-provider` line and one `tokio` line.
- [ ] `crates/runie-server/Cargo.toml` has exactly one `tokio` line.
- [ ] The surviving `tokio` line in each crate is the feature *union* of the two originals:
  - `runie-agent`: `["rt", "macros", "rt-multi-thread"]`
  - `runie-server`: `["rt-multi-thread", "macros", "io-std", "io-util", "net", "signal"]` (the `["rt", "macros"]` subset is absorbed)
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo build -p runie-server` succeeds (the server's `net` / `signal` features are preserved).
- [ ] `cargo build -p runie-agent` succeeds (the agent's `rt-multi-thread` feature is preserved).
- [ ] `cargo test --workspace` succeeds.
- [ ] No new `cargo` warnings about duplicate keys.

## Tests

### Layer 1 — State/Logic
- [ ] `manifest_has_no_duplicate_dep_keys` — a small test (or `cargo metadata` check) asserts each `[dependencies]` table key is unique in `runie-agent` and `runie-server` Cargo.toml.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_agent_still_uses_rt_multi_thread` — `cargo build -p runie-agent` succeeds and the agent binary still spawns a multi-thread runtime.
- [ ] `smoke_server_still_binds_tcp` — `cargo build -p runie-server` succeeds; the `TcpListener::bind` call site compiles (proves `net` feature is present).

## Files touched

- `crates/runie-agent/Cargo.toml` (merge 2 pairs into 2 single lines)
- `crates/runie-server/Cargo.toml` (merge 1 pair into 1 single line)

## Notes

Zero-risk. Run `cargo build --workspace` before and after to confirm feature union is preserved. If `cargo` ever starts erroring on duplicate keys (it has been discussed upstream), this task pre-empts that breakage.
