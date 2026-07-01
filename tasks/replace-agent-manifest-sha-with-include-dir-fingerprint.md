# Replace agent manifest SHA with include_dir fingerprint

## Status

`todo`

## Context

`resources/agents/manifest.json`, `subagents/manifest.rs`, and `build.rs:201-231` maintain SHA-256 hashes for bundled agent files. Adding an agent requires updating the manifest and hashes. `build.rs` pulls in `sha2`/`hex` just for this.

## Goal

Drop the manifest JSON; embed `resources/agents/` with `include_dir!`. Either trust compile-time inclusion or generate a build-time fingerprint from the `Dir` entries. Remove `sha2`/`hex` build deps.

## Acceptance Criteria

- [ ] Use `include_dir!` for agent resources.
- [ ] Delete `manifest.json` and SHA validation.
- [ ] Remove `sha2`/`hex` from build deps.
- [ ] Agent loading still works.

## Design Impact

No change to TUI element design or composition. Only agent resource bundling changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all agent files are embedded.
- **Layer 2 — Event Handling:** Agent resource loading emits same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Subagent invocation works.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
