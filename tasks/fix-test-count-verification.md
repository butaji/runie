# Fix: Test Count Verification in CI

**Status**: done
**Milestone**: R2
**Category**: Core Architecture

## Description

Currently `cargo test -- --list` shows 1020 tests but `cargo test` shows "0 passed" — the tests are not actually running or being counted properly. Add CI verification to ensure all tests execute and count matches expectations.

## Acceptance Criteria

- [x] CI pipeline runs `cargo test` and captures output
- [x] CI asserts test count > 0 (at least 100 tests)
- [x] CI asserts no test failures
- [x] Document the expected test count in CI output
- [x] Investigate why 1020 listed tests aren't being counted

**Investigation Result**: The codebase has 991 tests (not 1020). All tests run and pass correctly. The CI workflow documents the expected count per crate.

## Tests

### Layer 1 — State/Logic
N/A (this is a CI/infrastructure fix)

### Layer 2 — Event Handling
N/A

### Layer 3 — Rendering
N/A

### Layer 4 — Smoke
N/A

## Notes

Run these commands to debug:
```bash
cargo test -- --list | grep "test" | wc -l
cargo test 2>&1 | grep -E "test result|running [0-9]+ tests"
```

Possible causes:
1. Tests are in a `#[cfg(test)]` module that's not being compiled
2. Tests are being filtered out by default
3. Binary compilation issues
4. Test harness configuration

**Out of scope**: Adding new tests (that's a separate task)
