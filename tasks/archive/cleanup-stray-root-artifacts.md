# Cleanup stray root artifacts

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

A removed TUI parity/dspec workflow left behind ~190 git-tracked files and several stray root files that no code, CI, justfile, or dev.sh references. These pollute grep/glob searches and mislead readers. Delete them in one sweep.

## Acceptance Criteria

- [ ] `bin/runie-dspec-watch`, `bin/runie-parity`, `bin/runie-rerun` deleted (they build deleted binaries `scenario_replay`, `runie-dspec`, `runie-cli`).
- [ ] `ui/` directory removed (only fed the dead parity binaries + `scripts/watch-dspec.sh` + stale `ui/TUI_DEV.md`).
- [ ] `scripts/watch-dspec.sh` deleted (targets deleted `runie-dspec`).
- [ ] `.hermes/` removed (mutable scratch file `last-scenario.txt` tracked in git + 6 stale plan docs only referenced by dead bin scripts).
- [ ] `bacon.toml` stripped to live jobs (`ui`, `check`, `check-skip`) or deleted; dead jobs (`run`, `test`, `parity`, `replay`) removed.
- [ ] Root files `EOF` (0B), `run` (26B "hello - bash, doesnt send") untracked and deleted.
- [ ] Untracked one-shot scripts `fix_into.py`, `refactor_events.py`, `__pycache__/` deleted; `__pycache__/` added to `.gitignore`.
- [ ] `git status` shows no stray untracked root artifacts.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure file deletion, no logic.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_workspace_builds_after_artifact_cleanup` — `cargo check --workspace` green after deletions.
- [ ] `smoke_no_dead_bin_references` — `rg "scenario_replay|runie-dspec|runie-cli" .` returns zero hits outside `tasks/archive/`.

## Files touched

- `bin/runie-dspec-watch`, `bin/runie-parity`, `bin/runie-rerun`
- `ui/` (entire directory)
- `scripts/watch-dspec.sh`
- `.hermes/` (entire directory)
- `bacon.toml`
- `EOF`, `run`
- `fix_into.py`, `refactor_events.py`, `__pycache__/`
- `.gitignore`

## Notes

Common root cause: a single removed TUI parity/dspec workflow whose wrappers and artifacts were never cleaned up. Group everything into one commit. Do not touch `dev.sh` or `justfile` (verified live).
