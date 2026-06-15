# Delete crates/_archive/ Once Archived Code Is Confirmed Unneeded

**Status**: done
**Milestone**: R1
**Category**: Core / State
**Priority**: P1
**Depends on**: archive-remaining-orphans

## Description

After commits `402943c5`, `0959861e`, and `f831dea5`, the directory
`crates/_archive/` contains 30 files (9 subdirectories + 1 file)
of dead code. The intent was to preserve the broken/unbuildable
code in case anyone wanted to revive it later. But the archive is:

1. **Not in the build path** — confirmed by `git grep` returning
   zero hits in the live tree for the archived files
2. **Never referenced from the live code** — verified during the
   task reviews
3. **Causes confusion** — newcomers see `crates/_archive/` and
   wonder "is this used?"
4. **Inflates the `find crates` results** — `crates/_archive/`
   contains ~9,500 lines of dead code (1,852 from the
   about-to-be-deleted `loop_engine` is in the live tree; the
   archive is mostly broken tests, broken imports, and orphan
   crates)

The 9 subdirs + 1 file at `crates/_archive/`:

| Entry | Type | Source | Lines |
|---|---|---|---|
| `runie-agent-bin/` | directory | `reply_to_scenario.rs` (old binary, broken) | ~120 |
| `runie-agent-tests/` | directory | 8 test files referencing archived crates | ~880 |
| `runie-ext/` | directory | Extension system (lib, error, hooks, marketplace, mcp, registry) | ~476 |
| `runie-ext-macros/` | directory | Proc-macro crate for extensions | ~150 |
| `runie-tui-bins/` | directory | 5 test binaries (grok_parity_test, scenario_replay, scenario_fasthot, runie-dspec, runie-paint-smoke) | ~3,500 |
| `update-orphans/` | directory | 3 files (login_flow.rs, state.rs, integration.rs) — broken code archived by 0959861e | ~782 |
| `wireframe_layout.rs` | file | Old wireframe test file | ~124 |
| `runie-ai/` | directory | AI provider code (model_fetcher, providers, session_adapter, etc.) | ~600 |
| `runie-cli/` | directory | Full CLI implementation (4,104 lines) | ~4,104 |
| `runie-tools/` | directory | Tool implementations (bash, edit_file, read_file, etc.) | ~800 |

Total: ~9,500 lines of dead code.

## Acceptance Criteria

- [x] `git grep -rn 'crates/_archive' -- 'crates/' 'build.rs' 'Cargo.toml'`
  returns zero matches
- [x] `git log --oneline -- crates/_archive/` shows commits
  `402943c5`, `0959861e`, and `f831dea5` as the only commit
  history (plus any followups)
- [x] `git rm -r crates/_archive/` (or `rm -rf` and `git add -A`)
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` succeeds (no tests should reference
  the archive; current count is 1,569 tests)
- [x] The workspace `Cargo.toml` `members` array is unchanged
  (8 crates)
- [x] `build.rs` lint scans are now 30 files smaller

## Tests

### Layer 1 — State/Logic
- [x] `cargo build --workspace` succeeds after the deletion
- [x] `cargo test --workspace` succeeds
- [x] `git grep -rn 'crates/_archive' -- 'crates/' 'Cargo.toml' 'build.rs'`
  returns zero matches

### Layer 4 — Smoke
- [x] `cargo build` then `./target/release/runie` (or
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
git grep -nE 'crates/_archive' -- 'crates/' 'build.rs' 'Cargo.toml' \
  ':!crates/_archive/*'
# Expected: zero matches

# Step 2: confirm the archived files are git-tracked (not lost)
git log --oneline -- crates/_archive/ | head -5

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

**`runie-cli` is the largest archive entry** (4,104 lines). It
contains a complete alternative CLI implementation with ACP
support, headless mode, session management, and event logging. The
intent appears to have been to replace `runie-term` (which is only
457 lines). That replacement never happened; `runie-cli` is
abandoned.

**`runie-tui-bins` is the second-largest** (5 binaries, ~3,500
lines). These were test scaffolding binaries that referenced the
archived `runie-ext` and other broken code.

**Out of scope:**
- Restoring any of the archived code to the live tree (separate
  revival tasks; e.g. "re-enable MCP support" or "switch to
  runie-cli as the main binary")
- Reorganizing the surviving code (separate refactor tasks)
- Auditing the git history of each archived file to find the
  last-known-good state (use `git log -- crates/_archive/file.rs`
  before deletion if a revival is anticipated)
- The `runie-agent/src/loop_engine/` and `runie-agent/src/events.rs`
  dead code (different task: `delete-abandoned-loop-engine`)

**Verification:**
```bash
# Archive is gone
! test -d crates/_archive

# Build clean
cargo build --workspace
cargo test --workspace
```
