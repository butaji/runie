# Compare headless one-shot scenarios and fix gaps

**Status**: blocked

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo), Grok Build fixtures not present
**Milestone**: R7
**Category**: Testing
**Priority**: P0

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Run Grok Build headless (`grok --no-auto-update -p "..." --output-format json`) and `runie-headless print/json` for the same prompts. Identify output-shape, stop-reason, tool-use, and exit-code differences. Fix Runie gaps with unit + E2E tests.

## Scenario Set

1. Simple greeting: `"hello"`
2. File listing: `"list files"`
3. Bash tool: `"run echo hi"`
4. Code edit: `"add a Rust doc comment to src/lib.rs"`
5. Multi-step request: `"refactor the auth module to use JWT"`

## Acceptance Criteria

- [ ] Each scenario runs in both tools via the harness.
- [ ] Differences are classified as missing feature, dead-end, confusing UX, bug, or reference-only.
- [ ] For every actionable finding, a follow-up `tasks/<id>.md` is created (or an existing task is updated) with unit + E2E test AC.
- [ ] High-priority gaps (e.g. headless edit tool missing, infinite loops) are fixed before lower-priority ones.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] Per-fix unit tests as defined in each child task.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_headless_hello_parity` — outputs are comparable for `hello`.
- [ ] `harness_headless_list_files_parity` — tool use and final output are comparable.

## Files touched

- Determined by findings; likely `crates/runie-cli/src/print.rs`, `crates/runie-agent/src/headless/`, `crates/runie-core/src/headless_runtime.rs`.

## Fixture / Replay Strategy

This task must use recorded Grok Build headless fixtures (`crates/runie-testing/fixtures/grok-build/headless/`) produced by `scripts/record-grok-build-fixtures.sh`. Convert Grok's JSON/stdout output into Runie provider-replay fixtures. Do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The CLI headless native-tool loop (`fix-cli-headless-native-tool-permission-denied-loop`) is expected to surface here.
- Do not mutate the Runie repo; run tools in a temp fixture copy.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `AgentActor`/`TurnActor` own headless state.
- [ ] **Trigger events:** Headless prompts trigger agent processing.
- [ ] **Observer events:** Tool calls, responses emit events.
- [ ] **No direct mutations:** Headless processing must emit events, not mutate directly.
- [ ] **No new mirrors:** Headless state is authoritative in actors; no duplicates.
- [ ] **Async work observed:** Tool execution has JoinHandle owners.
