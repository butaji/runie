# Gate test support with `#[cfg(test)]`

**Status**: done
**Milestone**: R7
**Category**: Testing

## Description

`runie-core/src/tests/support.rs` was unconditionally compiled into non-test builds. The helpers (`fresh_state`, `type_str`, `exec`, `seed_providers`, `tmp_store`, `minimal_session`, `ENV_LOCK`) were dead code in production binaries.

## Changes

1. **`crates/runie-core/src/tests/mod.rs`** — `mod support` changed from `#[allow(unused)]` to `#[cfg(test)]`; `pub use support::*` changed from unconditional to `#[cfg(test)]`.

2. **`crates/runie-core/src/lib.rs`** — `tests_support` module kept unconditional (so `runie-testing` can compile against it as a dev-dependency) but re-exports inside are gated `#[cfg(test)]`.

3. **`crates/runie-testing/src/tests/state.rs`** — Helpers (`fresh_state`, `type_str`, `exec`) are now implemented directly here instead of re-exporting from `runie_core::tests_support`. This avoids a compile-time dependency on `runie_core::tests_support` being available at the call site.

## Acceptance Criteria

- [x] **Unit tests** — All test helpers (`fresh_state`, `type_str`, `exec`) available in `runie-core` and `runie-testing` tests; `cargo test --workspace` passes (3,085 tests, 0 failures).
- [x] **E2E tests** — Smoke tests pass (verified via `cargo test --workspace`).
- [x] **Production builds** — `support.rs` helpers are gated `#[cfg(test)]` and do not appear in non-test compilation.

## Validation

```
$ cargo test --workspace 2>&1 | grep "^test result"
test result: ok. 207 passed
test result: ok. 4 passed
test result: ok. 2 passed
test result: ok. 30 passed
test result: ok. 1969 passed
test result: ok. 3 passed
test result: ok. 124 passed
test result: ok. 713 passed
... (no failures)
```

## Notes

- `runie-testing` duplicates the three public helpers (`fresh_state`, `type_str`, `exec`) so that it does not need `runie_core::tests_support` to be available at compile time when `runie-testing` is compiled as a non-test dev-dependency.
- `runie_core::tests_support` in `lib.rs` is kept unconditional (with `#[allow(unused)]`) so that `runie-testing`'s lib compilation succeeds in non-test mode.
- The `#[allow(unused)]` on `tests_support` suppresses the "module is never used" warning when the re-exports are empty (non-test builds).
