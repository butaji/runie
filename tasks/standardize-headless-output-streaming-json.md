# Standardize headless output as streaming JSON events

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: expose-runie-via-acp-stdio
**Blocks**: none

## Summary

Replace the custom headless CLI output format with a single streaming JSON event stream. All clients — TUI, headless scripts, ACP consumers — read the same `Fact` shape.

## Acceptance Criteria

- Headless mode (`runie -p "..."`) emits newline-delimited JSON events.
- Event schema covers turn progress, tool calls, approvals, completions, and errors.
- Custom progress/formatting modules in `runie-cli` are removed.
- TUI can be switched to consume the same stream internally.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Snapshot tests for streaming JSON event serialization.
- **Layer 4**: Headless run with captured provider fixture produces expected JSON lines.
