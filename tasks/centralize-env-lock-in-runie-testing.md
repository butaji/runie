# Centralize ENV_LOCK in runie-testing

## Status

`done`

## Context

Several test modules define their own `static ENV_LOCK: Mutex<()>` to serialize env-var mutation:
- `crates/runie-core/src/tests/support.rs:17`
- `crates/runie-core/src/tests/copy.rs:12`
- `crates/runie-provider/src/tests.rs:20`
- `crates/runie-provider/src/config/tests.rs:6`
- `crates/runie-tui/src/tests/render/render_slash.rs:4`

## Goal

Move the lock and a helper like `env_lock()` into `runie-testing` and have all crates import it from one place.

## Acceptance Criteria

- [x] Add `pub static ENV_LOCK` and `env_lock()` helper to `runie-testing`.
- [x] Replace the duplicate statics in all crates.
- [x] All tests still pass.

## Implementation

1. Added `crates/runie-testing/src/env_lock.rs` with:
   - `pub static ENV_LOCK: Mutex<()>`
   - `pub fn env_lock<F, T>(f: F) -> T` helper

2. Updated exports in `runie-testing/src/lib.rs`

3. Updated all consumers:
   - `crates/runie-provider/src/tests.rs` — uses `runie_testing::{env_lock, ENV_LOCK}`
   - `crates/runie-provider/src/config/tests.rs` — uses `runie_testing::ENV_LOCK`
   - `crates/runie-core/src/tests/copy.rs` — uses `runie_testing::ENV_LOCK`
   - `crates/runie-tui/src/tests/render/render_slash.rs` — uses `runie_testing::ENV_LOCK`
   - `crates/runie-core/src/tests/support.rs` — re-exports `runie_testing::ENV_LOCK`

## Validation

- `cargo check --workspace` passes
- `cargo test -p runie-testing` passes (17 tests)
- All workspace tests still pass
