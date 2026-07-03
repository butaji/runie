# Grok Build Fixture Strategy — Minimize Live Invocations

## Problem

Grok Build is a paid, rate-limited, beta external service. Running it inside the normal `cargo test` or CI loop would be:

- **Slow** — network calls and model latency add seconds or minutes per scenario.
- **Expensive / rate-limited** — every invocation costs tokens and may hit quotas.
- **Flaky** — beta model behavior can vary between runs.
- **Blocking** — tests would fail if credentials are missing.

The comparison tasks must therefore treat Grok Build as a **reference**, not a runtime dependency. We capture its behavior once, store it as fixtures, and replay those fixtures in Runie's deterministic E2E tests.

## Strategy Overview

```
┌─────────────────┐     one-time or on-demand      ┌─────────────────────┐
│  Grok Build CLI │ ─────────────────────────────> │  Fixture store      │
│  (live/expensive)│    record scenario outputs     │  (text/json/panes)  │
└─────────────────┘                                └─────────────────────┘
                                                             │
                                                             │ load
                                                             ▼
┌─────────────────┐                                ┌─────────────────────┐
│  Runie tests    │ <──────────────────────────────│  Replay provider /  │
│  (fast/deterministic)                            │  mock harness       │
└─────────────────┘                                └─────────────────────┘
```

## Principles

1. **Never call Grok Build from `cargo test`.** Tests use fixtures.
2. **Record fixtures on demand, not per run.** A maintainer re-records only when the comparison scope changes or Grok Build behavior materially changes.
3. **Prefer headless recordings.** Headless mode (`grok -p`) is faster, cheaper, and easier to diff than TUI recordings.
4. **TUI recordings are one-time per scenario.** Use `tmux capture-pane` once, store the pane dump, and replay it as a visual reference.
5. **Derive Runie fixtures from Grok outputs.** Turn Grok's JSON/stdout/pane output into provider-replay fixtures and expected snapshots for Runie tests.
6. **CI does not need Grok credentials.** CI validates Runie against recorded fixtures.

## Fixture Store Layout

```
crates/runie-testing/fixtures/grok-build/
├── headless/
│   ├── hello/
│   │   ├── prompt.txt
│   │   ├── stdout.txt
│   │   ├── stderr.txt
│   │   ├── exit_code.txt
│   │   └── runie_expected.json
│   ├── list_files/
│   ├── run_bash/
│   └── ...
└── tui/
│   ├── launch/
│   │   ├── pane.txt
│   │   └── runie_snapshot_expected.txt
│   ├── hello_chat/
│   ├── slash_help/
│   └── ...
```

Each fixture directory contains the raw Grok output plus a derived artifact showing what Runie should produce.

## Recording Workflow

A single script, `scripts/record-grok-build-fixtures.sh`, performs the expensive live runs:

1. Reads a manifest (`scripts/grok-build-scenarios.toml` or embedded list).
2. For each scenario:
   - Creates a fresh temp copy of the fixture repo.
   - Runs the Grok Build command.
   - Captures stdout, stderr, exit code, and (for TUI) `tmux capture-pane` output.
   - Stores the output in `crates/runie-testing/fixtures/grok-build/`.
3. Produces a summary diff report.

The script must:
- Skip gracefully if Grok Build is not authenticated.
- Allow a `--scenarios` filter to re-record only a subset.
- Print cost/time estimates so the operator can decide.
- Never write into the Runie working directory.

## From Grok Fixtures to Runie E2E Tests

### Headless scenarios

- Parse Grok's `--output-format streaming-json` or `json` output.
- Map the event stream to Runie's provider events (`text`, `tool_call_start`, `tool_call_input_delta`, `tool_call_end`, `tool_result`, `end`).
- Build a provider-replay fixture in `crates/runie-testing/fixtures/provider-replay/grok-build-<scenario>.jsonl`.
- Write an E2E test that runs the same prompt through Runie headless with the replay provider and asserts the stdout matches Grok's output shape.

### TUI scenarios

- Store the captured pane as a visual reference.
- Derive the equivalent Runie `TestBackend` expected `Buffer`.
- Write a rendering test that feeds the same sequence of events to Runie and asserts the buffer matches.
- Where behavior differs by design, document the intentional difference instead of forcing pixel parity.

## Running Grok Build During Development

Developers should run Grok Build only in these situations:

1. **Initial fixture generation** — once per scenario set.
2. **Re-recording** — when Grok Build updates or the scenario changes.
3. **Manual spot-checks** — to validate a controversial finding.

Normal workflow:

```bash
# One-time expensive recording
scripts/record-grok-build-fixtures.sh

# Fast deterministic tests forever after
cargo test --workspace
```

## Comparison Harness Alignment

`scripts/compare-with-grok-build.sh` (planned in `build-runie-vs-grok-build-comparison-harness`) should:

1. Check for existing fixtures.
2. If fixtures are missing, optionally invoke the recorder.
3. Compare Runie output against the fixture, not against a live Grok run.
4. Support a `--live` flag for on-demand re-recording only.

## Task Checklist

- [ ] Create `scripts/record-grok-build-fixtures.sh`.
- [ ] Define the scenario manifest format.
- [ ] Record headless fixtures for all scenarios in the comparison matrix.
- [ ] Record TUI fixtures for all TUI scenarios.
- [ ] Convert Grok fixtures into Runie provider-replay fixtures.
- [ ] Convert TUI pane dumps into `TestBackend` expected buffers.
- [ ] Update comparison tasks to state that live Grok invocations happen only through the recorder.
- [ ] Document in `AGENTS.md` or `EXECUTE.md` that tests must not call external paid APIs.

## Notes

- This strategy mirrors the existing Layer-4 provider-replay pattern in `AGENTS.md`.
- The `grok-build` binary still needs to be runnable for recording, but CI and day-to-day development do not need it.
