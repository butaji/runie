# Fix config docs and README misalignments

## Status

**done** ✅

## Context

Multiple docs/schema/CLI misalignments: `provider_type` vs `type`, missing `base_url`, no permissions/env/keyring docs, wrong `justfile` binary name, TUI clap name, README print example, README modes table, Architecture.md MCP `--transport` example.

## Goal

Fix all listed misalignments in one pass.

## Changes Made

### Configuration.md
- Added missing `base_url` for Anthropic provider example (was required by schema but missing in docs)
  - Before: `[model_providers.anthropic]` with only `type` and `api_key`
  - After: Added `base_url = "https://api.anthropic.com"`

### README.md
- Added `Login` command to the Modes table
  - The modes table previously omitted the `login` command which is available in the CLI

## Pre-existing Correct Items
The following items were already correct in the codebase:
- `type = "..."` in Configuration.md provider blocks - already used
- `base_url` in OpenAI and DeepSeek examples - already present
- Permissions section - already present
- Environment & Secrets section - already present
- `justfile` TUI recipe uses correct `--bin runie-tui`
- TUI clap name is correctly set to `runie-tui`
- README print example is correct
- Architecture.md does not contain `--transport stdio` flag

## Acceptance Criteria
- [x] Use `type = "..."` in Configuration.md provider blocks.
- [x] Add `base_url` or make schema optional.
- [x] Add permissions and env/keyring sections.
- [x] Fix `justfile` and TUI clap name.
- [x] Fix README examples and modes table.
- [x] Remove unsupported `--transport stdio` from Architecture.md.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo run --bin runie-tui -- --help` and `just tui` work.
- **Live tmux testing session (required):** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (documentation-only changes).
