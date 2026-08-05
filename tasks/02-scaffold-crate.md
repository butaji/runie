# Step 02: Scaffold runie-core crate skeleton

**Status:** pending
**Depends on:** 01

## Goal
Create the `runie-core` crate with `Cargo.toml`, `src/lib.rs`, and empty module files for each subdirectory in the layout. Verify it compiles.

## Changes
- `crates/runie-core/Cargo.toml`: package metadata; deps `tokio`, `async-trait`, `serde`, `serde_json`, `futures`, `thiserror`, `tracing`.
- `crates/runie-core/src/lib.rs`: re-exports stubs.
- `crates/runie-core/src/types.rs`: empty module.
- `crates/runie-core/src/state/mod.rs`, `actor.rs`, `snapshot.rs`: empty.
- `crates/runie-core/src/queues/mod.rs`, `steering.rs`, `follow_up.rs`: empty.
- `crates/runie-core/src/loop/mod.rs`, `actor.rs`, `driver.rs`, `turn.rs`: empty.
- `crates/runie-core/src/tools/mod.rs`, `actor.rs`, `registry.rs`, `executor.rs`: empty.
- `crates/runie-core/src/provider/mod.rs`, `actor.rs`, `stream_fn.rs`: empty.
- `crates/runie-core/src/events/mod.rs`, `bus.rs`, `subscribe.rs`: empty.
- `crates/runie-core/src/convert.rs`: empty.

## Verification
- `cargo check -p runie-core` → exit 0.
- `cargo build -p runie-core` → exit 0.
- `find crates/runie-core/src -name '*.rs' | wc -l` → 16 (matches the layout).

## Notes
- Empty modules each contain only a `//! <name>` doc comment so they're real modules.