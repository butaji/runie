# runie

Rust port of [`@earendil-works/pi-agent-core`](https://github.com/earendil-works/pi) using
single-source-of-truth actors + events.

## Crates

| Crate        | Purpose                                                      |
|--------------|--------------------------------------------------------------|
| `runie-core` | Agent loop, state, events, tools, queues (the pi-port)       |
| `runie-tui`  | Actor/MVU TUI, YAML replay, and Grok parity snapshots       |
| `lint-check` | Build-script-style linter enforcing project rules            |

## Architecture

Events-based, single-source-of-truth actors:

- Each state slice is owned by exactly one actor.
- The only change mechanism is events published by the owning actor.
- Read-only projections / snapshots are rebuilt from events.
- Every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event).

| Actor                | Owns                                                       |
|----------------------|------------------------------------------------------------|
| `AgentStateActor`    | system prompt, model, messages, tools, streaming, pending  |
| `SteeringQueueActor` | mid-run steering messages                                  |
| `FollowUpQueueActor` | post-run follow-up messages                                |
| `ToolExecutorActor`  | tool call dispatch (sequential + parallel)                 |
| `ProviderActor`      | one in-flight LLM stream                                   |
| `LoopActor`          | the loop task + subscriber registry                        |
| `StatusActor`        | status, usage, waiting, and theme snapshot                  |
| `ScrollbackActor`    | transcript, tool cards, selection, and scroll               |
| `PromptActor`        | prompt, history, mode, and file-search state                |
| `UiActor`            | palette/modal state and typed UI actions                    |

## Event Sequence

Matches `pi-agent-core`'s README exactly:

```
prompt("X")
├─ agent_start
├─ turn_start
├─ message_start { userMessage }
├─ message_end   { userMessage }
├─ message_start { assistantMessage }
├─ message_update (assistant only, possibly many)
├─ message_end   { assistantMessage }
├─ [tool_execution_start/update/end + toolResult messages]
├─ turn_end   { message, toolResults: [] }
└─ agent_end  { messages: [...] }
```

`agent_end` listeners are awaited in registration order before the loop task resolves.

## Building & Testing

```bash
cargo check --workspace
cargo test  --workspace
cargo run   -p lint-check
cargo doc   -p runie-core --no-deps
```

Tests are layered: core replay/state tests, TUI unit tests, runtime-discovered
YAML scenarios, and full-screen asciinema/TestBackend comparisons. Run the
complete local gate with `just ci`.

## Tasks

Implementation tracked under `tasks/` as `index.json` + per-step `NN-name.md`
files. Status is updated as each step completes.

## Test Performance

The test suite uses `cargo-nextest` for parallel per-test execution. To get
fastest iteration:

```bash
just test-fast    # cargo nextest run --workspace
```

What we did to make tests fast:

| Change | Speedup |
|--------|---------|
| Switched to `cargo-nextest` (per-test process, parallel by default) | ~CPU-count× parallelism vs. cargo's serial test runner |
| Added `.config/nextest.toml` with `test-threads = "num-cpus"` and `failure-output = "immediate-final"` | maxes out CPU; cleaner failure output |
| Added `.cargo/config.toml` with `mold` linker on Linux targets | 1.5–2× faster linking (skipped on macOS) |
| Kept `[profile.dev]` with `codegen-units = 16`, `opt-level = 1`, `incremental = true` | faster rebuilds |
| Removed the redundant `tests/yaml_e2e.rs` integration test (the `runie-tui-e2e` binary covers the same path) | one fewer compile target per rebuild |

What we **didn't** do (and why):

- We **didn't** add `sccache` — not pre-installed on this host.
- We **didn't** enable LTO or `panic = "abort"` in dev profile — both slow down incremental rebuilds.
- We **didn't** split `runie-tui` tests into multiple binaries; the current
  workspace gate already exercises unit, replay, YAML, and visual paths.

## Current parity status

The local workspace gate is green. Core parity covers the pi event/state/tool
loop contract and provider replay; TUI parity covers actor-owned MVU state,
themes, prompt/scrollback/status/layout states, typed tool cards, workflow
lifecycle, YAML scenarios, and four-size visual captures. Remaining work is
tracked in `tasks/p16`, `p19`, `p21`, and the source inventory in `tasks/p30`.
