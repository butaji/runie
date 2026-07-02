# Add convention / lint against magic numbers

## Status

`todo`

## Description

Add a project convention to `AGENTS.md` and a lightweight CI check (e.g., grep for raw literals in new code or `clippy::numeric_literal` lint) to prevent new magic numbers.

## Acceptance criteria

1. **Unit tests** — A static check catches new raw literals in actor/provider/TUI code.
2. **E2E tests** — CI passes with the new check.
3. **Live tmux tests** — Not applicable; this is a process/lint task.

## Tests

### Unit tests
- Lint script identifies a known magic number in a test fixture.

### E2E tests
- CI workflow runs the check and reports zero new violations.

### Live tmux tests
- N/A (document as skipped).
