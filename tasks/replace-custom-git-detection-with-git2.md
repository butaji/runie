# Replace custom git detection with git2

## Status

`todo`

## Context

`crates/runie-core/src/actors/io/ractor_io.rs:283-378` implements manual `.git/HEAD` / `.git/config` parsing to detect branch and origin repo name. This is brittle for worktrees, packed refs, and symbolic refs. `git2` is already in the workspace transitively via `fff-search`.

## Goal

Replace the custom parser with `git2::Repository::discover`, `repo.head()`, and `repo.find_remote("origin").

**Design impact:** No change to TUI element design or composition. Only the accuracy of branch/origin metadata shown in the status bar changes.

## Acceptance Criteria

- [ ] Remove `detect_git_info_sync` and related manual parsing.
- [ ] Use `git2` to discover the repository, current branch, and origin URL.
- [ ] Keep the same `GitInfo` struct returned to callers.
- [ ] Add error handling for non-git directories and detached HEAD.

## Tests

- **Layer 1 — State/Logic:** Unit tests using temporary git repositories (worktree, detached HEAD, no origin).
- **Layer 1:** Non-git directory returns `None` without error.
- **Layer 2 — Event Handling:** `IoMsg::DetectGit` produces the expected `GitInfoLoaded` fact.
- **Layer 3 — Rendering:** `TestBackend` status bar shows branch/origin info in a git repo.
- **Layer 4 — E2E:** Headless CLI startup reports git context correctly.
- **Live tmux validation:** Open the TUI in a git worktree and in a normal repo; status bar shows the correct branch and origin.
