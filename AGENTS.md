# Agent Guidelines

All features, fixes, and improvements must be implemented with coverage of automatic tests that are lightweight and run fast: unit and e2e.
No artificial delays in automatic tests!

Each implementation must be live tested in a real terminal tmux session (or a live CLI/headless run for non-TUI tasks) to make sure everything is working as expected.

## Testing Strategy (4 Layers)

### Layer 1: State/Logic (Pure Functions)
Test business rules and state transitions without any Ratatui imports.

```rust
#[test]
fn counter_increments() {
    let mut app = App::default();
    app.tick();
    assert_eq!(app.counter, 1);
}
```

### Layer 2: Event Handling
Feed crossterm events directly into handlers.

```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[test]
fn q_quits() {
    let mut app = App::default();
    let event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    app.handle_event(event);
    assert!(app.should_quit);
}
```

### Layer 3: Rendering
Use TestBackend + Buffer assertions for widget tests.

```rust
use ratatui::{backend::TestBackend, Terminal, widgets::Paragraph};

#[test]
fn renders_hello() {
    let backend = TestBackend::new(10, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| {
        let widget = Paragraph::new("hello");
        f.render_widget(widget, f.area());
    }).unwrap();

    let expected = Buffer::with_lines(vec!["hello     ", "          ", "          "]);
    terminal.backend().assert_eq(&expected);
}
```

### Layer 4: Provider Replay / Mock-Tool E2E
Run the agent turn end-to-end with captured provider SSE fixtures and fake tool outputs. These tests catch the bugs lower layers cannot — async event ordering, race conditions, stale indices, inflight leaks, TurnComplete duplication, stuck timers — without shelling out or using tmux.

```rust
#[tokio::test]
async fn minimax_m3_multi_tool_turn() {
    use runie_agent::headless::{run_headless_turn, HeadlessOptions};
    use runie_core::event::Event;
    use runie_core::message::{ChatMessage, Role};
    use runie_testing::capture_events;
    use runie_testing::allow_all_gate;

    let fixtures = vec![
        include_str!("fixtures/minimax/m3_multi_tool_list_dir.sse").to_string(),
        include_str!("fixtures/minimax/m3_read_file_call.sse").to_string(),
        include_str!("fixtures/minimax/m3_read_file_final.sse").to_string(),
    ];
    let provider = runie_testing::dyn_replay_provider_with(&fixtures, "minimax", "MiniMax-M3");

    let (events, emit) = capture_events();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("list files and read README"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };

    run_headless_turn(messages, &provider, options).await.unwrap();

    let events = events.lock();
    assert!(events.iter().any(|e| matches!(e, Event::TurnComplete { .. })));
}
```

**When to run:** Before every push, in CI, or when changing async/event logic.
**What they catch:** Event reordering, stale indices, inflight leaks, TurnComplete duplication, stuck timers, and provider-specific parser regressions.

## Anti-Patterns (Never Do These)

| Don't | Why |
|-------|-----|
| Use shell or tmux tests | Prefer deterministic Rust tests with mock IO |
| Use `sleep()` in tests | Non-deterministic |
| Test widget internals | Test output, not structure |
| Mix state + rendering in one test | Hard to debug |

## Architecture Principles

Everything must be **events-based with SSOT actors**.

- **Single Source of Truth (SSOT):** Each runtime fact is owned by exactly one actor. The actor's state is the only authoritative copy.
- **Events are the change mechanism:** The only way to observe or react to a change is through events published by the owning actor (via the `EventBus` or actor channel).
- **No direct mutation:** Handlers, tools, subagents, and tests must not mutate another actor's state directly. Send a message; let the actor transition state and emit events.
- **No mirrored state:** If a second location holds the same data, it must be a read-only projection or snapshot rebuilt from events, never independently mutable.
- **Observed async work:** Every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event). No unbounded fire-and-forget `tokio::spawn`.

## File Structure

```
src/
├── app.rs      # State + logic (pure tests)
├── handler.rs  # Event mapping (input tests)
├── ui.rs       # Widgets + layout (render tests)
```

**Rule**: Your App should compile without ratatui if you strip rendering.

## Linter Rules

**ENFORCED GUARDRAILS**

The build script at `crates/runie-core/build.rs` enforces, across all workspace production `.rs` files:

| Check | Scope | Fail-on-violation |
|-------|-------|-------------------|
| AppState field access patterns | all `crates/*/src` production code | Yes |
| Magic numbers (>= 1000) | all `crates/*/src` production code | Yes |
| Orphan `tokio::spawn` calls | all `crates/*/src` production code | Yes |

`python3 scripts/check_structure.py` (run in CI) enforces the structural limits on all workspace production `.rs` files:

| Metric | Limit |
|--------|-------|
| File lines | ≤ 500 |
| Function lines | ≤ 40 |
| Approximate complexity | ≤ 10 |

Complexity is an approximate heuristic that counts `if`, `else if`, `match`, `while`, `for`, `loop`, `break`, `continue`, `return`, `&&`, `||`, and `?` tokens. It does not parse Rust syntax and may miss nested closures, match guards, and similar constructs.

**Known gaps / exemptions:**
- A number of legacy files and functions are temporarily exempt from the structural limits while they are refactored. The exemption list is in `scripts/check_structure.py`. New code should not be added to this list.
- Test code and `build.rs` files are exempt from structural limits so tests can stay comprehensive.

**AppState field access** ensures internal state fields are accessed through accessor methods, not directly.

**Magic number guardrail** prevents raw numeric literals (>= 1000) in production code. Numbers below 1000, underscore-separated numbers, hex literals, HTTP status codes, JSON-RPC error codes, and numbers in test code are exempt. Use named constants for buffer sizes, timeouts, and thresholds.

**GUIDELINES**

Keep files small, functions focused, and complexity low. When a function grows beyond ~60 lines, consider extracting helper functions. When a file exceeds ~400 lines, consider splitting or extracting modules.
