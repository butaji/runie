# Round 2 — Testing Strategy & Fixtures

## Findings

### 1. Environment-variable tests are not consistently serialized

- `runie-testing/src/env_lock.rs:9` provides a global `ENV_LOCK`, but many tests mutate `std::env` directly.
- There are **three** separate `ENV_LOCK` definitions: `runie-testing/src/env_lock.rs:9`, `crates/runie-core/src/tests/support.rs:17`, `crates/runie-core/src/tests/copy.rs:12`.

### 2. `temp_home()` isolation bug

- `crates/runie-testing/src/fixtures.rs:17-21` — uses `std::sync::Once`, so `HOME` is set only on the first call. Later calls return new temp dirs but `HOME` points at the first one.

### 3. Test scaffolding leaks into production builds

- `crates/runie-core/src/tests/mod.rs:14` re-exports test helpers unconditionally.
- `crates/runie-core/src/tests/support.rs:61-66` — `tmp_store()` sync filesystem cleanup compiled into non-test builds.

### 4. Heavy filesystem / environment dependence

- `crates/runie-core/src/tests/session_store.rs`, `crates/runie-core/src/tests/slash/save_load.rs`, `crates/runie-core/src/config/tests/mod.rs` rely on real temp dirs and `RUNIE_SESSIONS_DIR`.
- `crates/runie-core/src/auth/credential.rs:47-57` snapshots the entire process environment at construction.
- `crates/runie-provider/src/tests.rs:292-339` uses `127.0.0.1:1`, which is network-dependent.

### 5. Timing-dependent / slow tests

- `crates/runie-provider/src/tests.rs:159-178` — `RUNIE_MOCK_DELAY=1` triggers 300–800 ms real sleep.
- `crates/runie-provider/src/mock.rs:234-241` — `MockStreamingProvider` defaults to 10 ms delays with `< 50 ms` assertions.
- `crates/runie-provider/src/tests.rs:292-325` asserts wall-clock timeout behavior.

### 6. Snapshot / fixture brittleness

- `crates/runie-provider/tests/minimax_replay.rs` uses `insta::assert_debug_snapshot!` on full event vectors including raw reasoning text.
- `crates/runie-tui/src/tests/snapshots/*.snap` depend on `Debug` formatting.

## Recommended changes

1. Enforce the shared `runie_testing::ENV_LOCK` everywhere; delete duplicate locks.
2. Fix `temp_home()` to set/restore `HOME` per call or return a guard.
3. Gate `mod tests`/`tests_support` with `#[cfg(test)]`.
4. Provide in-memory `SessionStore`/`CredentialResolver` backends for unit tests.
5. Use deterministic/mock time or set delays to 0 in unit tests.
6. Assert canonical fields, not `Debug` snapshots.
7. Adopt `test-case` for parameterized provider/parser tests.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Unify env locks | `tasks/fix-env-lock-isolation-and-remove-duplicates.md` | **new** |
| Gate test support | `tasks/gate-test-support-with-cfg-test.md` | **new** |
| In-memory test backends | `tasks/add-in-memory-backends-for-unit-tests.md` | **new** |
| Remove real sleeps | `tasks/eliminate-real-sleeps-in-provider-tests.md` | **new** |
| Parameterized tests | `tasks/add-parameterized-tests-with-test-case.md` | **new** |
