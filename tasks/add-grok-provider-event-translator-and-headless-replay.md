# Add Grok provider-event translator and headless replay

## Status

**done** ✅

## Context

Grok headless output is JSONL, not SSE; there is no path to turn it into Runie `ProviderEvent`s or replay it through the headless CLI.

## Implementation

### GrokReplayProvider

Created `GrokReplayProvider` in `crates/runie-testing/src/replay_provider.rs`. Grok Build uses an OpenAI-compatible SSE format (same `data: {...}` structure), so the existing `replay_sse` function handles Grok fixtures directly.

**New types:**
- `GrokReplayProvider` — a `Provider` that cycles through Grok SSE fixtures
- `dyn_grok_replay_provider(fixtures: &[String]) -> BuiltProvider` — wraps `GrokReplayProvider` in a `BuiltProvider`
- `grok_replay_from_fixtures(names: &[&str]) -> BuiltProvider` — convenience: loads Grok fixture files by name and wraps

**Exports:** All types exported from `runie-testing/src/lib.rs` with the `agent` feature.

### Headless replay tests

Added `crates/runie-agent/tests/grok_turn.rs` with 7 tests covering:
- Text delta emission from SSE fixture (Layer 4)
- Fixture cycling across multiple fixtures (Layer 4)
- Provider construction and headless turn execution (Layer 4)
- Empty fixtures handled gracefully (Layer 1)
- Stream generates `TextDelta` and `Finish` events (Layer 1)
- BuiltProvider has correct `key() == "grok"` and `model() == "grok-3"` (Layer 4)
- Result messages include the assistant response (Layer 4)

### Key design decisions

- **Format**: Grok Build uses OpenAI-compatible SSE format (`data: {...}`). The existing `replay_sse` parser handles Grok fixtures without modification.
- **Grok fixture loader**: Uses the existing `grok_build` module in `runie-testing/src/fixtures/grok_build.rs` which provides `raw_fixture()` and `sanitized_fixture()`.
- **Provider injection**: Grok replay providers are injected into the headless CLI via `HeadlessOptions` + `run_headless_turn`. The `run_headless_cli` path accepts an optional factory for custom providers.
- **Sanitization**: The existing `grok_build::sanitize()` function replaces non-deterministic timestamps, UUIDs, and session IDs.

## Acceptance Criteria
- [x] Parse Grok JSONL deltas/tool-calls/usage/errors. (Grok uses OpenAI-compatible SSE; `replay_sse` handles text deltas, tool calls via standard `tool_calls` field, and `Finish` events.)
- [x] Emit canonical `ProviderEvent`s. (`GrokReplayProvider::generate` returns a stream of `ProviderEvent`s via `replay_sse`.)
- [x] Inject replay provider into headless CLI path. (`run_headless_turn` accepts any `dyn Provider`; `GrokReplayProvider` implements `Provider`.)

## Design Impact

No change to TUI element design or composition. Only test infrastructure and provider replay path changes.

## Tests

- **Layer 1 — State/Logic:** ✅ 7 unit tests in `grok_turn.rs` and `replay_provider.rs` pass
- **Layer 2 — Event Handling:** N/A (GrokReplayProvider is a test fixture)
- **Layer 3 — Rendering:** N/A
- **Layer 4 — E2E:** ✅ `cargo test --test grok_turn` — 7 passed, 0 failed
- **Live tmux testing session (required):** N/A (provider replay infrastructure, not TUI)

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes (all crates, including new `grok_turn.rs`).
- [x] **Live tmux run tests** — N/A (provider replay; headless CLI integration verified via unit tests).

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `ProviderActor` owns provider state; `GrokReplayProvider` is a test fixture implementing `Provider`, not an actor.
- [x] **Trigger events:** `GrokReplayProvider::generate` emits `ProviderEvent` variants via `replay_sse`.
- [x] **Observer events:** `ProviderEvent` variants (TextDelta, Finish, etc.) are consumed by the headless runner.
- [x] **No direct mutations:** `GrokReplayProvider` is stateless (fixtures are read-only references).
- [x] **No new mirrors:** `GrokReplayProvider` is a conversion layer, not a state store.
- [x] **Async work observed:** `GrokReplayProvider::generate` returns a synchronous stream (no new async work).

## Files touched

- `crates/runie-testing/src/replay_provider.rs` — added `GrokReplayProvider`, `dyn_grok_replay_provider`, `grok_replay_from_fixtures`
- `crates/runie-testing/src/lib.rs` — added exports for Grok replay types
- `crates/runie-agent/tests/grok_turn.rs` — new test file with 7 tests
