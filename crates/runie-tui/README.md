# `runie-tui`

Minimal ratatui/crossterm TUI for [`runie-core`](../runie-core).

## What it does

Single-screen chat interface:

```
┌─────────────────────────────────────────┐
│  scrollback transcript                  │
│                                         │
│  user> Hello                            │
│  assistant> Hi there!                   │
│  ⚙ bash: ... → ✓                        │
│                                         │
├─────────────────────────────────────────┤
│  > _                                    │  prompt input
├─────────────────────────────────────────┤
│  ready                                  │  status bar
└─────────────────────────────────────────┘
```

Subscribes to `runie-core`'s `EventBus` and mutates widgets on each
`AgentEvent`. No auth, MCP, ACP, sub-agents, voice, plan-approval,
worktrees, dashboard, persona picker, themes — by design.

## Key bindings

| Key       | When              | Effect                          |
|-----------|-------------------|---------------------------------|
| Enter     | prompt non-empty  | submit to loop                  |
| Ctrl+C    | streaming         | abort the current run           |
| Ctrl+C    | idle              | quit                            |
| Ctrl+D    | any               | quit                            |
| Ctrl+L    | any               | clear scrollback                |
| Esc       | prompt non-empty  | clear prompt                    |

## Run

```bash
cargo run -p runie-tui
```

The current binary uses a placeholder `StreamFn` that emits one canned
response. Wiring a real provider (`runie-provider` adapter) is a
follow-up task.

## Tests

```bash
cargo test -p runie-tui
```

The integration test in `tests/e2e_test.rs` drives the `App` via
`TestBackend`, runs a `MockStreamFn` through `runie-core`, and asserts
the rendered Buffer contains the expected transcript.

## E2E scenarios (YAML)

YAML scenarios live in `tests/e2e/*.yaml`. Editing or adding a YAML file
does **not** require rebuilding — the `runie-tui-e2e` binary loads them
at runtime.

```bash
just e2e                  # run every tests/e2e/*.yaml
just e2e-one tool-echo    # run one specific scenario
cargo run -p runie-tui --bin runie-tui-e2e -- tests/e2e/hello-streaming.yaml
```

Each scenario defines:

- `initial_prompt` (optional)
- `follow_up` (list of messages to enqueue)
- `tools` (echo, …)
- `events` (sequence of mock `AssistantMessageEvent`s: `start`, `text_delta`, `tool_call`, `done`, `error`)
- `assertions`:
  - `transcript_contains` (substring checks against the rendered scrollback)
  - `events` (event-kind sequence must contain each named kind)
  - `turn_starts` (exact count of `turn_start` events)
  - `scrollback_lines` (per-kind lines with substring matches)
  - `visual` (in-process `TestBackend` render → `screen_text` / `screen_excludes`
    substring assertions; mirrors grok-build's pty `harness.screen_contents()`
    contract but runs in-process — no pty, no real binary).

Example:

```yaml
name: hello-streaming
initial_prompt: "say hi"
events:
  - start
  - text_delta: "Hello"
  - text_delta: " world"
  - done: { stop_reason: stop }
assertions:
  transcript_contains:
    - "Hello world"
  events: [agent_start, turn_start, message_start, message_end, turn_end, agent_end]

---

name: visual-minimal
description: Welcome modal at idle mirrors grok's minimal-mode chrome.
events: []
assertions:
  visual:
    cols: 80
    rows: 24
    screen_text:
      - "Runie"
      - "Model · runie-core"
      - "/help for commands"
      - "session_start"
      - "❯"
      - "ready"
    screen_excludes:
      - "user>"
      - "assistant>"
      - "panicked"
```

## Snapshot tests (insta)

The crate uses `insta::assert_snapshot!` for pure-function output
(adopted from grok-build's `src/app/status_blocks.rs` pattern). Snapshots
live in `src/snapshots/`. To update after intentional changes:

```bash
cargo install cargo-insta   # one-time
cargo insta review          # interactively accept/reject
```

Example (`event_renderer::tests::welcome_modal_snapshot`):

```rust
#[test]
fn welcome_modal_snapshot() {
    let text: String = super::welcome_modal_lines()
        .iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    insta::assert_snapshot!("welcome_modal", text);
}
```

## Visual test patterns adopted from grok-build

| Pattern | Source in grok-build | Adopted in runie-tui |
|---|---|---|
| In-process `TestBackend` render with substring assertions | pty-driven `tests/pty_e2e/*` (the in-process equivalent) | `render_visual()` in `src/yaml_runner.rs` + `assertions.visual` in YAML |
| `insta::assert_snapshot!` of pure-function output | `src/app/status_blocks.rs`, `src/scrollback/blocks/tool/edit.rs` | `event_renderer::tests::welcome_modal_snapshot` |
| YAML-defined test fixtures (load at runtime, no rebuild) | — (grok uses Rust fixtures) | `tests/e2e/*.yaml` + `runie-tui-e2e` binary |
| Synchronous event replay (sidesteps runtime scheduling) | n/a — this was a fix needed in runie | recorder + oneshot stop signal in `yaml_runner.rs` |

Patterns **not yet applicable** (runie-core doesn't have these features):

- pty harness (grok's `xai-grok-testing` crate)
- Commit-pipeline unit tests (`xai-grok-pager-minimal/commit_tests.rs`)
- Diff-block format snapshots (`xai-grok-pager/src/scrollback/blocks/tool/edit.rs`)
- Cost-tracking snapshots (`xai-grok-pager/src/app/status_blocks.rs`)