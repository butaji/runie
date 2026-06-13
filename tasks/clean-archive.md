# Delete crates/_archive/ Once Archived Code Is Confirmed Unneeded

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1
**Depends on**: archive-remaining-orphans

## Description

After commits `402943c5` and `0959861e`, the directory
`crates/_archive/` contains 28 .rs files + 2 .toml files of dead
code. The intent was to preserve the broken/unbuildable code in case
anyone wanted to revive it later. But the archive is:

1. **Not in the build path** — the lint scans it anyway (see
   `fix-build-lint`), and a 28-file graveyard makes `git grep`
   noisier
2. **Never referenced from the live code** — verified by `git grep`
   during the review: zero call sites in the live tree
3. **Causes confusion** — newcomers see `crates/_archive/` and
   wonder "is this used?"

The 28 files break down as:
- `crates/_archive/runie-ext/` (5 files): old extension system
- `crates/_archive/runie-ext-macros/` (1 file + Cargo.toml): proc
  macros for the old extension system
- `crates/_archive/runie-ext-macros/Cargo.toml`: manifest
- `crates/_archive/runie-agent-tests/` (8 files): old test files
  that referenced archived crates
- `crates/_archive/runie-agent-bin/reply_to_scenario.rs`: old binary
- `crates/_archive/runie-tui-bins/` (5 files): old test binaries
  (grok_parity_test, scenario_replay, scenario_fasthot, runie-dspec,
  runie-paint-smoke)
- `crates/_archive/update-orphans/` (3 files): broken
  `update/login_flow.rs` that called missing `build_login_stack`,
  and 2 test files
- `crates/_archive/wireframe_layout.rs`: old wireframe test file

Total: ~6,381 lines of dead code.

## Acceptance Criteria

- [ ] Confirm `git grep` returns zero hits in the live tree for
  every archived file
- [ ] Check git history for any of the archived files that were
  **referenced by recent commits** (e.g. within the last 30 days).
  If any were, leave them in the archive until the references age
  out
- [ ] `rm -rf crates/_archive/` (or `git rm -r crates/_archive/`)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds (no tests should reference
  the archive)
- [ ] `cargo build --workspace --tests` succeeds
- [ ] The workspace `Cargo.toml` `members` array is unchanged
  (8 crates)
- [ ] `build.rs` lint scans are now 28 files smaller

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds after the deletion
- [ ] `cargo test --workspace` succeeds
- [ ] `git grep -rn 'crates/_archive' -- 'crates/' 'Cargo.toml' 'build.rs'`
  returns zero matches

### Layer 4 — Smoke
- [ ] `cargo build` then `./target/release/runie` (or
  `cargo run -p runie-term --bin runie`) starts the TUI without
  panicking

## Notes

**Before deleting, run the verification step.** The previous commits
archived the files *because* they were broken. If the live tree
silently depends on the archive (e.g. via a stale `use` statement
that the compiler doesn't catch because of a feature flag), the
build will fail.

```bash
# Step 1: confirm no live references
git grep -nE 'crates/_archive|runie_ext::|runie_ext_macros::|runie_tui_bins::' \
  -- 'crates/runie-core/' 'crates/runie-term/' 'crates/runie-tui/' \
  'crates/runie-agent/' 'crates/runie-provider/' 'crates/runie-print/' \
  'crates/runie-json/' 'crates/runie-server/' 'Cargo.toml' 'build.rs'
# Expected: zero matches

# Step 2: confirm the archived files are git-tracked (not lost)
git log --oneline -- crates/_archive/ | head -10

# Step 3: dry-run the deletion
git rm -r --dry-run crates/_archive/

# Step 4: actually delete
git rm -r crates/_archive/

# Step 5: build
cargo build --workspace
cargo test --workspace
```

**If the live tree DOES depend on the archive** (unlikely but
possible), the deletion fails. In that case:
- The dependency should be removed from the live tree first
- Or the archived file should be restored to its original location
  (and the original task that archived it undone)

**The `build.rs` lint will be faster** because it walks 28 fewer
files. Minor win but real.

**The `Cargo.lock` may shrink** if any of the archived crates had
transitive deps. Probably not, since the archived crates were
already not in the workspace and their deps were not in the lock
file (or were orphans in the lock file that get pruned).

**Out of scope:**
- Restoring any of the archived code to the live tree (separate
  revival tasks; e.g. "re-enable MCP support")
- Reorganizing the surviving code (separate refactor tasks)
- Auditing the git history of each archived file to find the
  last-known-good state (use `git log -- crates/_archive/file.rs`
  before deletion if a revival is anticipated)

**Verification:**
```bash
# Archive is gone
! test -d crates/_archive

# Build clean
cargo build --workspace
cargo test --workspace

# The lint is faster
time cargo build  # subjective, but should be slightly faster
```
