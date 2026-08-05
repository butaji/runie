# Step 13: fmt + clippy + lint-check sweep

**Status:** pending
**Depends on:** 12

## Goal
Run the three lint passes required by `AGENTS.md` and fix every finding.

## Changes
- No new code; only fixes.
- Run `cargo fmt --all` and commit any reformats.
- Address each `cargo clippy` finding.
- Address each `cargo run -p lint-check` finding (AppState accessor, magic number >=1000, orphan `tokio::spawn`).

## Verification
- `cargo fmt --all -- --check` → exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0.
- `cargo run -p lint-check` → exit 0.

## Notes
- If the linter script doesn't yet exist, step 01 set it up; this step is about reaching green, not scaffolding.
- Common findings to expect: magic numbers in tests (named constants), unwrap in tests (acceptable but flagged sometimes — handle per linter policy).