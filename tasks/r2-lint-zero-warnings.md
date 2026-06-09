# Lint Zero Warnings

**Status**: todo
**Milestone**: R2
**Category**: Code Quality

## Description

Eliminate all clippy warnings and prevent regressions.

## Current Warnings

```
warning: unused import: `TruncatedOutput`
warning: unused import: `crate::event::Event`
warning: unused import: `Role`
warning: unused variable: `thumb`
warning: function `timestamps_are_monotonic` is never used
warning: matching on `Some` with `ok()` is redundant
warning: this `map_or` can be simplified
```

## Acceptance Criteria

- [ ] `cargo clippy --all-targets` produces zero warnings
- [ ] Add `#[warn(clippy::all)]` to lib.rs files
- [ ] CI runs `cargo clippy --all-targets -- -D warnings`

## Notes

All warnings are in test code. Fix by removing unused imports/variables or prefixing with `_`.
