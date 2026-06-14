# Spike: Replace Custom Fuzzy Matcher with `nucleo`

**Status**: stale
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2
**Depends on**: crate-replacement-audit

## Resolution

Not started. `crate-replacement-audit` is done but this spike was never executed.
The custom fuzzy matcher in `crates/runie-core/src/fuzzy.rs` still uses the original
algorithm. `nucleo` is a more sophisticated matcher (used by Helix editor) but adds
a non-trivial dependency. The current fuzzy matcher is adequate for command palette
usage (dozens to low hundreds of items). The spike has not been warranted.

Archived in tasks/archive/ as stale — low-priority performance investigation that doesn't
address a user-visible problem.
