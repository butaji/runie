# Replace custom diff parser with `diffy`

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Summary

Delete `crates/runie-core/src/diff.rs` and use `diffy` (or `flickzeug` for fuzzy patch support) to parse and apply unified diffs.

## Acceptance Criteria

- `diffy` is added to workspace dependencies.
- Custom `diff.rs` is removed.
- All diff parsing and patch-application callers use `diffy::Patch::from_str` and `diffy::apply`.
- Edge cases (binary patches, drifted line numbers) are handled or explicitly documented as unsupported.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Parse sample unified diffs and assert correct hunks/ranges.
- **Layer 1**: Apply patches to base text and assert final content.
- **Layer 4**: Tool-replay test using a tool that emits diff output.
