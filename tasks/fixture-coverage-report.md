# Fixture coverage report

## Objective

Provide a single command that reports which black-box tests cover every recorded
SSE fixture under `runie-tests/fixtures/`, and produce a structured coverage
report that includes model names, protocols, scenario categories, and error
categories.

## Why this matters

The suite currently guarantees that all 131 fixtures are wired by at least one
test, but there is no quick way to audit *which* test covers *which* fixture, or
to see coverage grouped by model/protocol/scenario. A coverage report is a
hard requirement for proving public-release quality of the replay test matrix.

## Required implementation

1. Add a new Rust binary target or `xtask` script in `runie-tests/` named
   `fixture-coverage`.
2. The tool must:
   - Enumerate every `*.sse` file under `fixtures/openai/` and
     `fixtures/anthropic/`.
   - Search `tests/*.rs` for fixture filename references.
   - For each fixture, list the test file(s) and test function(s) that reference
     it.
   - Detect and report any unwired fixtures.
   - Group fixtures by:
     - protocol (`openai` / `anthropic`)
     - model name (parsed from filename, e.g. `deepseek_v4_pro`)
     - scenario (`simple`, `tool`, `multi_tool`, `reasoning`, `multiturn_*`,
       `rate_limit_error`, `server_error`, `status_*`, etc.)
     - error category for error fixtures (`rate_limit`, `server_error`,
       `context_length`, `invalid_key`, `model_not_found`, `http_status`)
3. Output a Markdown report to `docs/fixture-coverage.md` and a JSON report to
   `target/fixture-coverage.json` when run.
4. Expose the command through `just fixture-coverage`.

## Acceptance checklist

- [x] `just fixture-coverage` runs successfully and returns exit code 0.
- [x] The report lists every fixture and at least one covering test.
- [x] Unwired fixtures are reported as failures.
- [x] The Markdown report includes tables for protocol, model, scenario, and
      error-category counts.
- [x] The command completes in under 10 seconds without spawning tmux or
      building runie binaries (194 ms).

## Dependencies

- `recorded_trace_coverage` (done)
- `black_box_replay_testing` (done)
