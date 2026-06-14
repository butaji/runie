# Delete Committed `libstreaming_buffer.rlib` Build Artifact

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

## Description

A 265 KB Rust static library, `libstreaming_buffer.rlib`, was tracked by git at the
repository root. Build artifacts should live in `target/`, which is already gitignored.

## Acceptance Criteria

- [x] `libstreaming_buffer.rlib` is removed from the working tree and from git history
  (via `git rm`).
- [x] `*.rlib` is added to `.gitignore` so future artifacts cannot be committed.
- [x] `git ls-files | grep '\.rlib$'` returns nothing.

## Tests

No code changes. Verification:

```bash
ls libstreaming_buffer.rlib   # expected: No such file
git ls-files | grep '\.rlib$' # expected: empty
grep '^\*.rlib$' .gitignore   # expected: match
```

## Files touched

- `libstreaming_buffer.rlib` (delete)
- `.gitignore`

## Out of scope

- General `.gitignore` audit; only `*.rlib` is added here.
