# Runie Agent Harness

> **Status:** Task definitions have been cleaned up. Behavioral requirements previously stored in `harness/tasks/` and `crates/runie-agent/src/harness/tasks/` are now consolidated in `TASKS_FINDINGS_PLAN.md` at the workspace root.

End-to-end evaluation suite for the Runie TUI coding agent. Based on
[SWE-bench](https://github.com/princeton-nlp/SWE-bench) and
[Terminal Bench 2.0](https://github.com/terminal-benchmarks/terminal-bench) patterns.

All graders are written in Rust — no Python dependency.

---

## Quick Start

```bash
# Run all tasks via Rust library
cargo test -p runie-harness

# Run a specific integration test
cargo test -p runie-harness --test empty_state
cargo test -p runie-harness --test panic_recovery
```

---

## Structure

```
harness/
├── README.md              # This file
├── Cargo.toml             # Rust harness library
├── src/
│   ├── lib.rs             # Task discovery, execution, CSV output
│   └── graders/
│       └── static_analysis.rs  # Rust grader utilities
└── tests/                 # Rust integration tests (one per task)
    ├── empty_state.rs
    ├── panic_recovery.rs
    └── ...
```

The previous `tasks/` directory of JSON task definitions has been removed. The findings and planned work derived from those tasks now live in `TASKS_FINDINGS_PLAN.md`.

---

## Rust Library

The `src/lib.rs` provides a programmatic API:

```rust
use runie_harness::{HarnessConfig, run_all_tasks};

let config = HarnessConfig::default().verbose();
let result = run_all_tasks(&config).await;
println!("Pass rate: {:.0}%", result.pass_rate() * 100.0);
println!("{}", result.to_csv());
```

> **Note:** With the task directories removed, `run_all_tasks()` currently returns an empty result set. Future task definitions should be added as Rust integration tests under `harness/tests/` or as unit tests in the relevant crates.

---

## Metrics Tracked

| Metric | Description |
|--------|-------------|
| Task resolution rate | Pass/fail per task |
| Elapsed time | ms per task |
| Check counts | Individual assertions per task |
| Token cost | When model is configured |

---

## CSV Output Format

```csv
task_id,status,elapsed_ms,checks_passed,checks_total,detail
empty_state,pass,4,4,4,4/4 checks passed
error_state_recovery,pass,3,5,5,5/5 checks passed
```

---

## Adding a New Task

1. Create a Rust integration test under `harness/tests/<task_id>.rs`.
2. Add the behavioral requirement to `TASKS_FINDINGS_PLAN.md` if it represents new planned work.
3. Run: `cargo test -p runie-harness --test <task_id>`

---

## Constraints

- No new dependencies without justification
- All code must compile with `cargo check --all-targets`
- Tasks validate behavior, not implementation (black-box testing)
- Graders are Rust tests that assert on source code patterns
