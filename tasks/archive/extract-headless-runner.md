# Extract a Shared Headless Runner for Non-TUI Binaries

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

## Description

`runie-print`, `runie-json`, and `runie-server` each duplicate the same bootstrap:
load config, build provider, build system prompt, stream response, and (for print/json)
run tools. This should live in one shared runner so the binaries are thin CLI adapters.

## Acceptance Criteria

- [x] A new `run_headless_turn` helper exists in `crates/runie-agent/src/headless.rs`
  and takes messages, a provider, and `HeadlessOptions`.
- [x] `runie-print`, `runie-json`, and `runie-server` use the helper.
- [x] The duplicated default tool list string is now `runie_core::prompts::DEFAULT_TOOLS`.
- [x] All three binaries still pass their existing tests.

## Tests

### Layer 1 — State/Logic
- [x] `headless_runner_with_mock_returns_content`.
- [x] `headless_runner_executes_tool_and_returns_output`.

### Layer 2 — Event Handling
- [x] No event changes.

### Layer 3 — Rendering
- [x] No rendering changes.

### Layer 4 — Smoke
- [x] `runie-print "hello"` with `RUNIE_MOCK=1` prints the mock response.

## Files touched

- `crates/runie-agent/src/` (new `headless.rs` or additions to `lib.rs`/`turn.rs`)
- `crates/runie-print/src/main.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`
- `crates/runie-core/src/prompts.rs` (tool list constant)

## Out of scope

- Changing the JSON-RPC protocol or CLI argument interface.
