# Remove Vendored Third-Party Reference Repositories

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

## Description

The working tree contains three full git copies of unrelated open-source agent frameworks:
`autogen/` (~198 MB), `crewAI/` (~457 MB), and `SuperAGI/` (~72 MB). They are not
submodules, are gitignored, and no crate or script references them. They add ~727 MB
to the repository and create noise in searches.

## Acceptance Criteria

- [x] `autogen/`, `crewAI/`, and `SuperAGI/` are removed from the working tree.
- [x] `.gitignore` contains rules that prevent them from being re-added.
- [x] `cargo build --workspace` still passes.
- [x] `cargo test --workspace` still passes.

## Tests

No code behavior changes, so no new Rust tests are required. Verification is manual:

```bash
du -sh .
ls -d autogen crewAI SuperAGI 2>/dev/null   # expected: no output
git ls-files | grep -E '^(autogen|crewAI|SuperAGI)/'   # expected: empty
cargo build --workspace
cargo test --workspace
```

## Files touched

- `.gitignore`
- `autogen/` (delete)
- `crewAI/` (delete)
- `SuperAGI/` (delete)

## Out of scope

- Adding submodules or external references; if reference code is needed later, clone it outside the repo.
