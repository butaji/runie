# Runie Agent Harness

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
├── tests/                 # Rust integration tests (one per task)
│   ├── empty_state.rs
│   ├── panic_recovery.rs
│   └── ...
└── tasks/                 # Task definitions (JSON only)
    └── <task_id>/
        └── task.json      # Task definition
```

---

## Task Format

Each task lives in `tasks/<task_id>/` and contains only a `task.json`:

```json
{
  "id": "task_id",
  "name": "Human-readable name",
  "description": "What this task validates",
  "setup": {
    "files": {
      "path/to/file.rs": "// Source file content"
    }
  },
  "expected": {
    "check_name": true
  }
}
```

The grader is a Rust integration test in `harness/tests/<task_id>.rs`.

---

## Available Tasks

### UX / Dead-ends
| Task | Description |
|------|-------------|
| `empty_state` | Empty chat shows informative placeholder |
| `no_model_warning` | Status bar warns when no model configured |
| `idle_submit_feedback` | Empty submit shows feedback message |
| `progressive_disclosure` | Advanced options hidden by default |

### Error Handling
| Task | Description |
|------|-------------|
| `error_state_recovery` | Agent errors return to Chat, allow continuation |
| `panic_recovery_test` | Tool panics caught, workspace rolled back |
| `stream_error_partial_response` | Streaming errors handled gracefully |
| `streaming_garbage` | UTF-8 validation on streamed tokens |

### State / Transitions
| Task | Description |
|------|-------------|
| `state_transition_test` | All TuiMode transitions are explicit and valid |
| `cancellation_clean_state` | Ctrl+C leaves clean state |
| `ctrl_c_test` | Ctrl+C interrupts running agent |
| `permission_rollback` | Denied permission rolls back changes |

### Idempotency / Concurrency
| Task | Description |
|------|-------------|
| `idempotency_test` | Re-running same tool call doesn't double-execute |
| `double_submit_dedup` | Double submit is blocked with feedback |
| `workspace_concurrent_edits` | File locking prevents lost updates |
| `file_stale_edit` | File changes during edit are detected |

### Network / Tools
| Task | Description |
|------|-------------|
| `network_retry` | Network errors have retry/backoff logic |
| `graceful_degradation` | Component failures don't crash the app |
| `channel_backpressure_test` | Event channel handles overflow |

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

1. Create `harness/tasks/my_new_task/task.json` with task definition
2. Create `harness/tests/my_new_task.rs` with Rust grader tests
3. Run: `cargo test -p runie-harness --test my_new_task`

---

## Constraints

- No new dependencies without justification
- All code must compile with `cargo check --all-targets`
- Tasks validate behavior, not implementation (black-box testing)
- Graders are Rust tests that assert on source code patterns
