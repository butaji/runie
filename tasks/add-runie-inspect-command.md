# Add `runie inspect` command

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none (removed)
**Blocks**: none

## Summary

Add a `runie inspect` command that prints the runtime configuration discovered for the current directory. Because everything is declared in files, the system can show exactly what it loaded.

## Output sections

```bash
runie inspect
runie inspect --json
```

Human-readable sections:
- ✅ Config sources and layers (global + local)
- ✅ Loaded skills (user + bundled)
- ✅ Registered slash commands
- ✅ Built-in subagent types
- ⏳ MCP servers (not yet implemented — MCP server registry not exposed in runie-core)
- ✅ Permission rules
- ⏳ Active actor states (not implemented — inspect is read-only)
- ✅ Model catalog entries
- ⏳ Recent slash commands (MRU) (not implemented)

Secrets (API keys, tokens) are redacted in all output.

## Implementation

- `crates/runie-cli/src/inspect.rs` — new file with `InspectReport` struct and CLI logic
- `crates/runie-cli/src/main.rs` — added `inspect` subcommand
- `crates/runie-cli/Cargo.toml` — added `dirs` dependency

## Acceptance Criteria

- [x] `runie inspect` prints a human-readable summary.
- [x] `runie inspect --json` emits machine-readable JSON.
- [x] The command is read-only and never mutates state or starts a turn.
- [x] Secrets are redacted (API keys not included in `ProviderInfo`).
- [x] `cargo check --workspace` is green.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `inspect_report_builds_without_panic` — report builds without errors
- [x] `inspect_report_json_serializes` — JSON output is valid
- [x] `inspect_report_human_does_not_panic` — human output renders
- [x] `skill_info_contains_path` — skill paths are captured
- [x] `provider_info_has_no_api_key` — API keys are not exposed

### Layer 2 — Event Handling
- [x] Command handler routes to `inspect::run()`

### Layer 3 — Rendering
- N/A (CLI output, not TUI)

### Layer 4 — Smoke
- [x] `runie inspect` runs successfully
- [x] `runie inspect --json` produces valid JSON

## Notes

- MCP servers section is omitted because the MCP server registry is not exposed from runie-core; this can be added later when `runie mcp` CLI is implemented.
- Active actor states are intentionally omitted — the command is read-only and actor state inspection would require runtime access.
- MRU commands are not captured — this would require persistence layer changes.
