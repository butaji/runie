# Archive Remaining Orphan Crates (runie-ai, runie-cli, runie-tools)

**Status**: done
**Completed in**: f831dea5 ("archive: move remaining orphan crates to crates/_archive/")
**Milestone**: MVP
**Category**: Configuration
**Priority**: P1
**Depends on**: wire-orphan-crates

## Description (historical)

After `wire-orphan-crates` (commit `402943c5`), three orphan crate
directories remained in `crates/` but were **not in the
workspace**. They were pure dead code that confused `cargo
metadata`, IDEs, and `find` results.

| Directory | Files | Cargo.toml? | Notes |
|---|---|---|---|
| `crates/runie-ai/` | 8+ .rs files (no manifest) | ❌ No | `model_fetcher/`, `model_registry.rs`, `providers/`, `session_adapter.rs`, `token_usage.rs`, `unified_api.rs` |
| `crates/runie-cli/` | `Cargo.toml` + 5+ .rs files in `src/` + `tests/` | ✅ Yes, but broken | `Cargo.toml` depends on `runie-ai` and `runie-tools`, which have no Cargo.tomls |
| `crates/runie-tools/` | 8+ .rs files (no manifest) | ❌ No | `bash.rs`, `edit_file.rs`, `read_file.rs`, `registry.rs`, `rig_tools/`, `search.rs`, `workspace.rs`, `write_file.rs` |

## Resolution

The commit `f831dea5` archived all three:

- `crates/runie-ai/` → `crates/_archive/runie-ai/`
- `crates/runie-cli/` → `crates/_archive/runie-cli/`
- `crates/runie-tools/` → `crates/_archive/runie-tools/`

`cargo build --workspace` succeeds with the same 8 workspace
members. The 28 .rs files of orphan code are now in `_archive/`.

## `crates/_archive/` Final State

After this commit, `crates/_archive/` contains:

| Entry | Type | Source |
|---|---|---|
| `runie-agent-bin/` | directory | `reply_to_scenario.rs` (old binary) |
| `runie-agent-tests/` | directory | 8 test files (events, tools, turn, safety, subagent_test, parser, mod) |
| `runie-ext/` | directory | Extension system (lib, error, hooks, marketplace, mcp, registry) |
| `runie-ext-macros/` | directory | Proc-macro crate for the extension system |
| `runie-tui-bins/` | directory | 5 test binaries (grok_parity_test, scenario_replay, scenario_fasthot, runie-dspec, runie-paint-smoke) |
| `update-orphans/` | directory | 3 files (login_flow.rs, state.rs, integration.rs) — broken code archived by 0959861e |
| `wireframe_layout.rs` | file | Old wireframe test file |
| `runie-ai/` | directory | 6+ .rs files (model_fetcher, providers, session_adapter, etc.) |
| `runie-cli/` | directory | Full CLI implementation (4,104 lines, includes tui_run/, acp.rs, headless.rs, etc.) |
| `runie-tools/` | directory | 8+ .rs files (bash, edit_file, read_file, etc.) |

Total: 9 subdirs + 1 file = 30 files, ~6,400 lines of dead code
(plus the `runie-cli` orphan at 4,104 lines and the 3,000+ lines
of other archive content).

## Status

✅ Done. Workspace builds. Tests pass. Next step: `clean-archive`
(delete the entire `_archive/` directory).

## Followups

- `clean-archive` — verify no live refs and delete
  `crates/_archive/`
- `sync-docs` — update README and any other doc that referenced
  `runie-ai`, `runie-cli`, or `runie-tools` as features
