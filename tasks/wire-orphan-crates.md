# Wire Orphan Crates Into the Workspace

**Status**: done (partial)
**Completed in**: 402943c5 ("Wire orphan crates: archive unbuildable crates and update workspace")
**Milestone**: MVP
**Category**: Configuration
**Priority**: P0

## Original Description

Six crate directories existed under `crates/` but were missing from
`Cargo.toml`'s `[workspace] members = [...]`: `runie-ai`, `runie-cli`,
`runie-tools`, `runie-ext`, `runie-ext-macros`, and an extra
`runie-tui-bins` (parity test binaries).

## Resolution

The commit `402943c5` took the **archive** path:

- `runie-ext` (5 files: lib.rs, error.rs, hooks.rs, marketplace.rs,
  mcp.rs, registry.rs) was moved to `crates/_archive/runie-ext/`
- `runie-ext-macros` (Cargo.toml + src/lib.rs proc-macro crate) was
  moved to `crates/_archive/runie-ext-macros/`
- `runie-agent-tests/` and `runie-agent-bin/` (old test files and
  binaries that referenced archived crates) were moved to
  `crates/_archive/`
- `runie-tui-bins/` (5 binaries: scenario_fasthot, runie-paint-smoke,
  runie-dspec, grok_parity_test, scenario_replay) was moved to
  `crates/_archive/runie-tui-bins/`

Workspace `Cargo.toml` `members` array is **unchanged** (still 8
crates). The `build.rs` lint allow-list was updated to reflect the
archived files.

## What's NOT done (followup)

Three orphan crate directories are still in `crates/`:

- `crates/runie-ai/` — has 8+ .rs files (`model_fetcher/`,
  `model_registry.rs`, `providers/`, `session_adapter.rs`,
  `token_usage.rs`, `unified_api.rs`) but **no Cargo.toml**. Cannot
  be built standalone.
- `crates/runie-cli/` — has a `Cargo.toml` that depends on `runie-ai`
  and `runie-tools` (which don't have Cargo.tomls). Cannot be built
  even if added to the workspace.
- `crates/runie-tools/` — has 8+ .rs files (`bash.rs`, `edit_file.rs`,
  `read_file.rs`, `registry.rs`, `rig_tools/`, `search.rs`,
  `workspace.rs`, `write_file.rs`) but **no Cargo.toml**.

These three (48 .rs files total) are now a new category: code with no
build target. See `archive-remaining-orphans` for the cleanup.

## Status

✅ Done for the archived crates (5 directories moved). ⚠️ Three
remaining orphans need a followup task (`archive-remaining-orphans`).

**Update (commit f831dea5, "archive: move remaining orphan crates
to crates/_archive/"):** the 3 remaining orphan crates
(`runie-ai`, `runie-cli`, `runie-tools`) have been archived. See
`archive-remaining-orphans` for the full resolution.

## Followups

- `archive-remaining-orphans` — **done** in commit f831dea5
- `clean-archive` — eventually delete `crates/_archive/` entirely
  once we're confident nothing in it is needed
