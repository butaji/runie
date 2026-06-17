# Remove Artificial Delays from Automatic Tests

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

## Description

`AGENTS.md` forbids `sleep()` in tests. However, several test suites
still use sleeps:

- `crates/runie-core/src/config_reload.rs` (inline tests):
  `tokio::time::sleep(Duration::from_secs(4))` ×2,
  `tokio::time::sleep(Duration::from_secs(3))`.
- `crates/runie-core/src/config_reload/tests.rs`:
  `tokio::time::sleep(Duration::from_secs(4))` ×2,
  `tokio::time::sleep(Duration::from_secs(3))`.
- `crates/runie-term/tests/e2e_legacy.rs` and the new
  `crates/runie-term/tests/e2e/` suite use many
  `std::thread::sleep(...)` calls.

These sleeps make the suite slow and flaky. E2e tests are allowed to be
non-deterministic crash/smoke tests, but unit and integration tests must
not sleep.

## Acceptance Criteria

- [ ] All `tokio::time::sleep` calls are removed from
  `runie-core` library and integration tests.
- [ ] `config_reload/tests.rs` is either wired into the build or deleted
  (it is currently an orphan file — see `remove-orphan-modules.md`).
- [ ] E2e tests document why sleeps are unavoidable and are run with
  `--ignored` / a separate command.
- [ ] `cargo test --workspace` runtime is reduced.
- [ ] `cargo test --workspace` still passes.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo test -p runie-core --lib config_reload` passes without
  sleeping.

### Layer 2 — Event Handling
- [ ] `config_watcher_emits_on_change` uses a channel/notify instead of
  sleep.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] E2e smoke tests remain available via
  `cargo test -p runie-term --test e2e -- --ignored`.

## Notes

**Strategy for config reload:**
Use a deterministic test hook that triggers the watcher or a channel
that the watcher notifies.

**Strategy for e2e:**
Keep sleeps but gate behind `#[ignore = "e2e: ..."]` and document in
`AGENTS.md` that Layer 4 smoke tests are exempt from the no-sleep rule.

**Out of scope:**
- Rewriting e2e tests to be deterministic (they are intentionally
  black-box crash tests).

## Verification

```bash
grep -R "tokio::time::sleep\|std::thread::sleep" crates/runie-core/src --include='*.rs'
# Expected: no matches in non-e2e code

cargo test --workspace
```
