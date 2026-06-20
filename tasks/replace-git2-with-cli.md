# Replace git2 crate with git CLI via IoActor

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: finish-io-migration
**Blocks**: none

## Description

`git2` is a workspace dep with `vendored-libgit2`, which pulls a C toolchain into the build. It is used in 6 files, primarily for git status / branch / worktree detection. The codebase already has an `IoActor` that owns bash execution, and `git` is a universally available OS tool. Replace `git2` calls with `git` CLI invocations through `IoActor` (or a `GitService` trait backed by CLI). This removes the C dependency, simplifies the build, and aligns with "OS features" over vendored libraries.

## Acceptance Criteria

- [ ] Audit complete: list all 6 files using `git2::` and the specific APIs used (`Status`, `Repository`, branch detection, worktree detection, etc.).
- [ ] `GitService` trait (already in `actors/io/git.rs`) extended with a CLI-backed implementation that shells out to `git` via `IoActor`.
- [ ] All `git2::` calls replaced with `GitService` trait calls (production) or CLI parsing (tests).
- [ ] `git2` removed from `[workspace.dependencies]` and all crate `Cargo.toml`s.
- [ ] `vendored-libgit2` feature and its C build dependency gone from `Cargo.lock`.
- [ ] `cargo build --workspace` succeeds without the C toolchain step.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `git_service_cli_detects_branch` — CLI-backed `GitService` returns the same branch as `git2` did.
- [ ] `git_service_cli_detects_worktree` — worktree detection matches `git rev-parse --is-inside-work-tree` / `git worktree list`.
- [ ] `git_service_cli_handles_no_repo` — outside a git repo, `GitService` returns `None` (no panic, no error).

### Layer 2 — Event Handling
- [ ] `io_actor_git_command_emits_event` — `IoActor` runs `git` and emits `GitDetected` with parsed output.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_git_detection_in_real_repo` — in a temp git repo, `GitService` CLI impl returns correct branch + worktree status.
- [ ] `smoke_build_without_c_toolchain` — `cargo build --workspace` completes without invoking a C compiler for libgit2.

## Files touched

- `crates/runie-core/src/actors/io/git.rs` — add CLI-backed `GitService` impl
- `crates/runie-core/src/actors/io/actor.rs` — wire `git` command execution
- All 6 files using `git2::` (grep-driven; replace with `GitService` calls)
- `Cargo.toml` (root) — remove `git2` from `[workspace.dependencies]`
- Crate `Cargo.toml`s — remove `git2` deps
- `Cargo.lock` — regenerated (libgit2 transitive deps gone)

## Notes

Depends on `finish-io-migration` so the `IoActor` is the sole bash/command owner before adding `git` to it. The `git` CLI is universally available on all target platforms (macOS, Linux, Windows with Git Bash). Parsing `git` output is simpler than maintaining a C library binding. Rejected alternative: keep `git2` for "type safety" — rejected because the 6 call sites do simple reads (status, branch, worktree), not complex git operations; the type safety is not worth a C toolchain dependency. The `fff-search` crate may also use `git2` internally — verify and, if so, that is upstream and out of scope.
