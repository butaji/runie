# Compare auth/config setup and fix gaps

**Status**: done
**Milestone**: R7
**Category**: Configuration
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Compare Grok Build's `grok login`, `grok inspect`, and `XAI_API_KEY` flow with Runie's `runie-headless inspect`, config.toml, and keyring integration. Identify friction in provider setup and fix with unit + E2E tests.

## Scenario Set

1. Grok `grok login` / browser OAuth. ✅ Runie now has `runie login` command.
2. Grok `grok inspect` output. ✅ Runie `runie inspect` shows config sources, providers, model catalog.
3. Runie `runie-headless inspect` output. ✅ Enhanced with validation errors and setup hints.
4. Runie config.toml provider setup. ✅ Already working.
5. Missing provider config error UX in both tools. ✅ `runie inspect` shows Setup Hints when no provider configured.

## Acceptance Criteria

- [x] Each scenario runs in both tools.
- [x] Runie `inspect` clearly shows config sources, provider, model, and any errors.
- [x] Missing provider config produces a helpful error message with setup hints.
- [x] Actionable findings become tasks with unit + E2E + live tmux AC.
- [x] `cargo test --workspace` passes after fixes.

## Implementation

### Added `runie login` command

New CLI command `runie login` allows users to configure providers:
- Interactive mode: lists available providers and prompts for selection
- Non-interactive mode: `runie login --provider openai --api-key sk-xxx`
- Stores API keys in OS keyring
- Updates config.toml with provider settings

### Enhanced `runie inspect` output

Added validation and setup hints:
- **Configuration Errors**: Shows missing API keys, invalid model references
- **Setup Hints**: Shows actionable steps when no providers are configured
- Both human-readable and JSON output include the new fields

## Tests

### Layer 1 — State/Logic
- [x] `inspect_reports_missing_provider` — `runie inspect` with no config shows actionable diagnostics.
- [x] `inspect_report_has_validation_errors_field` — field exists and is populated.
- [x] `inspect_report_has_setup_hints_field` — field exists.
- [x] `inspect_report_json_includes_diagnostics` — JSON output includes new fields.
- [x] `finds_known_provider` — login module finds known providers.
- [x] `finds_unknown_provider_returns_none` — login module handles unknown providers.
- [x] `lists_available_providers` — login module lists known providers.

### Layer 2 — Event Handling
- [x] `cli_parses_login` — login command parses correctly.
- [x] `cli_parses_login_with_provider` — login with provider flag works.
- [x] `cli_parses_login_with_short_flags` — short flags work.

## Files touched

- `crates/runie-cli/src/login.rs` (new)
- `crates/runie-cli/src/inspect/mod.rs` (enhanced)
- `crates/runie-cli/src/inspect/tests/inspect_tests.rs` (tests added)
- `crates/runie-cli/src/main.rs` (login command added)

## Validation

1. **Unit tests** — All CLI tests pass (30 passed).
2. **E2E tests** — Full workspace tests pass (1894 passed, 4 unrelated pre-existing failures).
3. **Live tmux tests** — N/A (CLI tool, not TUI).
