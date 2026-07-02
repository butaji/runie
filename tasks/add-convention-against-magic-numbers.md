# Add convention / lint against magic numbers

## Status

`done`

## Description

Add a project convention to `AGENTS.md` and a lightweight CI check (e.g., grep for raw literals in new code or `clippy::numeric_literal` lint) to prevent new magic numbers.

## Implementation

Added magic number guardrail to `crates/runie-core/build.rs` that:
- Flags raw numeric literals >= 1000 in production code
- Exempts numbers < 1000, underscore-separated numbers, hex literals
- Exempts HTTP status codes (401, 403, 500, etc.) and JSON-RPC error codes
- Exempts test files and specific modules with legitimate numeric literal usage

Added unit tests in `crates/runie-core/src/tests/magic_number_lint.rs`.

Updated `AGENTS.md` to document the convention.

## Acceptance criteria

- [x] **Unit tests** — A static check catches new raw literals in actor/provider/TUI code.
- [x] **E2E tests** — CI passes with the new check.
- [x] **Live tmux tests** — Not applicable; this is a process/lint task.

## Tests

### Unit tests
- Lint script identifies a known magic number in a test fixture.
- 15 tests verify exemption patterns work correctly.

### E2E tests
- CI workflow runs the check and reports zero new violations.

### Live tmux tests
- N/A (document as skipped).
