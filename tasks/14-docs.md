# Step 14: Top-of-crate docs + project README

**Status:** implemented; living parity inventories remain active (2026-08-07)
**Depends on:** 13

## Goal
Document the crate and the port for future contributors.

## Changes
- `crates/runie-core/src/lib.rs`: prepend a `//!` module doc that quotes the `pi-agent-core` README event-sequence diagrams and points to the test suite as the behavioural contract.
- `README.md`: project-level README explaining what `runie-core` is, the port's behavioural contract, and how to run tests.
- `crates/runie-core/PORT_NOTES.md`: short note on differences from the TS original (custom messages use trait extension instead of declaration merging; provider layer is behind `StreamFn`; barrier semantics enforced identically).

## Verification
- `cargo doc -p runie-core --no-deps` → exit 0 with no broken links.
- `cargo readme -p runie-core` (if `cargo-readme` available) or manual check that the lib doc renders.

## Notes
- Documentation is the final step because everything else is "what does it do"; this is "what does it say it does".
