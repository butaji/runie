# Replace custom git detection with git2

## Status

`done`

## Context

`crates/runie-core/src/actors/io/ractor_io.rs:283-378` implemented manual `.git/HEAD` / `.git/config` parsing to detect branch and origin repo name. This was brittle for worktrees, packed refs, and symbolic refs. `git2` is already in the workspace via `fff-search`.

## Goal

Replace the custom parser with `git2::Repository::discover`, `repo.head()`, and `repo.find_remote("origin").

**Design impact:** No change to TUI element design or composition. Only the accuracy of branch/origin metadata shown in the status bar changes.

## Acceptance Criteria

- [x] Remove `detect_git_info_sync` and related manual parsing. — **Done**; manual parsing replaced with `git2::Repository::discover`.
- [x] Use `git2` to discover the repository, current branch, and origin URL. — **Done**; `ractor_io.rs:332-376` uses `git2::Repository::discover`, `repo.head().shorthand()`, `repo.find_remote("origin").url()`.
- [x] Keep the same `GitInfo` struct returned to callers. — **Done**; `GitInfo` struct unchanged.
- [x] Add error handling for non-git directories and detached HEAD. — **Done**; handled via `Option` returns and `git2::Error` mapping.

## Tests

- **Layer 1 — State/Logic:** Unit tests using temporary git repositories (worktree, detached HEAD, no origin).
- **Layer 1:** Non-git directory returns `None` without error.
- **Layer 2 — Event Handling:** `IoMsg::DetectGit` produces the expected `GitInfoLoaded` fact. *(covered by existing IoActor tests)*
- **Layer 3 — Rendering:** `TestBackend` status bar shows branch/origin info in a git repo. *(covered by existing rendering tests)*
- **Layer 4 — E2E:** `cargo test --workspace` includes the new git detection tests.
- **Live tmux validation:** Open the TUI in a git worktree and in a normal repo; status bar shows the correct branch and origin. *(manual verification)*

## Implementation

Replaced `read_branch_sync`, `read_origin_repo_name_sync`, `read_git_info_sync`, and `read_worktree_git_info_sync` with `git2::Repository::discover`, `repo.head().shorthand()`, and `repo.find_remote("origin").url()`. Worktree detection uses `repo.is_worktree()` and `repo.path()`.

Four unit tests added covering: real repo detection, non-git directory, temp repo with no origin, and detached HEAD state.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
