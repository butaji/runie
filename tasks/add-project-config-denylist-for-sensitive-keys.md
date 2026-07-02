# Add project config denylist for sensitive keys

## Status

`done`

## Context

Codex denies dangerous keys in project-local config files to prevent unsafe sharing.

## Goal

Add a project-local config denylist for keys like `model_providers`, `openai_base_url`, `profile`.

## Acceptance Criteria
- [x] Define denylist.
- [x] Reject or warn when project config contains denied keys.
- [x] Document precedence and restrictions.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for denied-key detection.
- **Layer 2 — Event Handling:** Config-loaded fact excludes/flags denied keys.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Project-layer tests pass.
- **Live tmux testing session (required):** N/A.

## Implementation

Added `PROJECT_CONFIG_DENYLIST` constant in `crates/runie-core/src/config/layers.rs` with sensitive keys:
- `api_key`, `api-key`, `apiKey` — credentials
- `base_url`, `base-url`, `baseUrl`, `openai_base_url` — server endpoints
- `model_providers`, `providers`, `models` — provider config
- `profile`, `permission_mode` — security-sensitive

The `collect_denied_keys` function recursively traverses TOML tables and arrays to find denied keys at any nesting level (e.g., `[providers.foo].base_url`).

`parse_and_check_denylist` reads the project config file, checks for denied keys, and emits a `tracing::warn!` for each one found. The config is still merged (warn-only, not reject) to avoid breaking existing workflows.

## Files Changed

- `crates/runie-core/src/config/layers.rs` — Added denylist, recursive key checker, and warning emission.

## Validation

- ✅ `cargo check --workspace` passes
- ✅ `cargo test -p runie-core layers` — 5 new unit tests pass
- ✅ `cargo test --workspace` — full test suite passes
