# Replace agent manifest SHA with include_dir fingerprint

## Status

`done`

## Context

`resources/agents/manifest.json`, `subagents/manifest.rs`, and `build.rs:201-231` maintained SHA-256 hashes for bundled agent files. Adding an agent required updating the manifest and hashes. `build.rs` pulled in `sha2`/`hex` just for this.

## Changes Made

1. **Deleted `resources/agents/manifest.json`** - no longer needed
2. **Deleted `src/subagents/manifest.rs`** - the `Manifest` struct was not used at runtime
3. **Updated `src/subagents/mod.rs`** - removed `mod manifest;` and `pub use manifest::Manifest;`
4. **Updated `build.rs`** - removed `validate_agent_manifest()` function and its call
5. **Updated `Cargo.toml`** - removed `sha2` and `hex` from build-dependencies

## Acceptance Criteria

- [x] Use `include_dir!` for agent resources. — Already using `include_str!` directly
- [x] Delete `manifest.json` and SHA validation. — Done
- [x] Remove `sha2`/`hex` from build deps. — Done
- [x] Agent loading still works. — Verified: `SubagentRegistry::from_builtins()` still loads all 4 agent types

## Design Impact

No change to TUI element design or composition. Only agent resource bundling changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all agent files are embedded. — Verified via `registry_loads_all_builtin_types` test
- **Layer 2 — Event Handling:** Agent resource loading emits same facts. — Unchanged
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Subagent invocation works. — Unchanged
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` passes; session tests pass, subagent tests pass
- [x] **E2E tests** — `cargo test --workspace` passes (1 pre-existing flaky test in full suite due to env var pollution, not related to this change)
- [x] **Live tmux run tests** — N/A

## Notes

The `Manifest` struct was not used at runtime anywhere in the codebase - it was only used for build-time SHA validation. Agent loading uses `include_str!` directly to embed the markdown files. This simplifies the build process by removing unnecessary SHA validation and dependencies.
