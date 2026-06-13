# Archive Remaining Orphan Crates (runie-ai, runie-cli, runie-tools)

**Status**: todo
**Milestone**: MVP
**Category**: Configuration
**Priority**: P1
**Depends on**: wire-orphan-crates

## Description

After `wire-orphan-crates` (commit `402943c5`), three orphan crate
directories remain in `crates/` but are **not in the workspace** and
**not in `crates/_archive/`**. They are pure dead code that
confuses `cargo metadata`, IDEs, and `find` results.

| Directory | Files | Cargo.toml? | Notes |
|---|---|---|---|
| `crates/runie-ai/` | 8+ .rs files (no manifest) | ❌ No | `model_fetcher/`, `model_registry.rs`, `providers/`, `session_adapter.rs`, `token_usage.rs`, `unified_api.rs` |
| `crates/runie-cli/` | `Cargo.toml` + 5+ .rs files in `src/` + `tests/` | ✅ Yes, but broken | `Cargo.toml` depends on `runie-ai` and `runie-tools`, which have no Cargo.tomls |
| `crates/runie-tools/` | 8+ .rs files (no manifest) | ❌ No | `bash.rs`, `edit_file.rs`, `read_file.rs`, `registry.rs`, `rig_tools/`, `search.rs`, `workspace.rs`, `write_file.rs` |

Total: 48 .rs files of unbuildable code. The `runie-cli` Cargo.toml
references them by path:

```toml
[dependencies]
runie-core = { path = "../runie-core" }
runie-ai = { path = "../runie-ai" }      # ← broken
runie-agent = { path = "../runie-agent" }
runie-tools = { path = "../runie-tools" } # ← broken
```

If `runie-cli` were added to the workspace, the build would fail with
"no Cargo.toml in `../runie-ai`".

## Acceptance Criteria

- [ ] `crates/runie-ai/` is moved to `crates/_archive/runie-ai/`
- [ ] `crates/runie-cli/` is moved to `crates/_archive/runie-cli/`
  (both the `Cargo.toml` and `src/`, `tests/` directories)
- [ ] `crates/runie-tools/` is moved to `crates/_archive/runie-tools/`
- [ ] `ls crates/` shows only the 8 workspace-member crate directories
  plus `crates/_archive/`
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo metadata --format-version=1 --no-deps` reports the same
  number of `workspace_members` as before (8)
- [ ] The archived `runie-cli` keeps its `Cargo.toml` so the
  historical record of "what was the intended dependency graph"
  survives (the file is small, ~50 lines)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds with the same test count as
  before the move (the archived code is not built, so no tests are
  lost; but we want to confirm nothing in the live code referenced
  the to-be-archived code)
- [ ] `cargo metadata --format-version=1 --no-deps | jq -r '.workspace_members[]' | wc -l` returns 8

### Layer 4 — Smoke
- [ ] `cargo build` then `./target/release/runie` (or `cargo run -p
  runie-term --bin runie`) starts the TUI without panicking

## Notes

**Why archive rather than wire:** the missing `Cargo.toml` files for
`runie-ai` and `runie-tools` are non-trivial to write because the
code references types from other crates (`runie-core`, `runie-agent`)
that have evolved. Writing the manifest would require:
1. Reconciling the `runie-ai` `model_registry.rs` with the actual
   `runie-provider/src/model.rs` (which is the live version)
2. Reconciling the `runie-tools` tool implementations with the live
   `runie-agent/src/tools.rs`
3. Writing a manifest that depends on the now-current versions of
   `runie-core` and `runie-agent`

This is days of work for code that has been sitting in the tree
unused. The pragmatic call (made by `wire-orphan-crates`) is to
archive. This task extends that pattern to the remaining three.

**Verify nothing in live code references the to-be-archived files:**

```bash
# Should return zero matches
git grep -nE 'runie_ai::|runie_tools::' -- 'crates/runie-core/' \
  'crates/runie-term/' 'crates/runie-tui/' 'crates/runie-agent/' \
  'crates/runie-provider/' 'crates/runie-print/' 'crates/runie-json/' \
  'crates/runie-server/'
```

**The 4 parity-test binaries in `_archive/runie-tui-bins/` are
also there** from the previous commit. Not in scope here; they may
be deleted by `clean-archive` or `clean-dead-modules`.

**Out of scope:**
- Reconciling `runie-ai` model registry with the live
  `runie-provider` (would require a separate refactor task; the
  archived code is a snapshot, not a working artifact)
- Reconciling `runie-tools` with `runie-agent/src/tools.rs` (same)
- Restoring the `runie-cli` binary (the live binary is in
  `runie-term/src/main.rs`; `runie-cli/src/main.rs` is a duplicate
  of unknown provenance)
- Adding the 3 archived crates back to the workspace later (a
  separate "revival" task if/when the code is needed)

**Verification:**
```bash
ls crates/
# Should show: _archive  runie-agent  runie-core  runie-json  runie-print
#               runie-provider  runie-server  runie-term  runie-tui
# (8 members + _archive; no runie-ai, no runie-cli, no runie-tools)

cargo build --workspace
cargo test --workspace
```
