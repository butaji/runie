# Agent Guidelines

Track tasks in tasks/index.json, details per each in tasks/xxx.md

All features, fixes, and improvements must be implemented with coverage of automatic tests that are lightweight and run fast.
No artificail delays in automatic tests!

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

### Layer 4: Smoke & Crash Tests (tmux)
Run the real binary inside tmux, feed keys via `tmux send-keys`, capture pane output, assert no panics/stuck timers/crashes. These are **not** deterministic unit tests — they find bugs the layers above cannot: async event ordering, race conditions, stale indices, infinite loops.

```bash
#!/bin/bash
set -e
BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_$$"
LOG="/tmp/runie_smoke_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.3

# Type and submit
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
sleep 1.0

# Resize stress
for i in $(seq 1 10); do
    tmux resize-window -t "$SESSION" -x $((20 + i * 6)) -y $((5 + i * 2))
    sleep 0.05
done

# Rapid submit
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
sleep 3.0

# Capture and check
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Assert no stuck timers (elapsed > 1000s means infinite loop)
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "STUCK TIMER!"; exit 1
fi

# Assert no panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "PANIC!"; exit 1
fi

echo "Smoke test passed"
```

**When to run:** Before every push, in CI, or when changing async/event logic.
**What they catch:** Event reordering, stale indices, inflight leaks, TurnComplete duplication, stuck timers, memory leaks in long-running sessions.

## Anti-Patterns (Never Do These)

| Don't | Why |
|-------|-----|
| Use tmux tests as replacement for Layers 1-3 | They complement, not replace, fast unit tests |
| Use `sleep()` in tests | Non-deterministic |
| Test widget internals | Test output, not structure |
| Mix state + rendering in one test | Hard to debug |

## File Structure

```
src/
├── app.rs      # State + logic (pure tests)
├── handler.rs  # Event mapping (input tests)
├── ui.rs       # Widgets + layout (render tests)
```

**Rule**: Your App should compile without ratatui if you strip rendering.

## Task Authoring Rules

Every task in `tasks/<id>.md` must include a `## Tests` section with acceptance
criteria that reference the 4 testing layers. A task is **not done** until all
listed tests pass.

**Template:** See `tasks/TEMPLATE.md`.

**Required test coverage per category:**

| Category | Required Layers |
|----------|-----------------|
| Core / State | Layer 1 + Layer 2 |
| Tools | Layer 1 |
| TUI / Rendering | Layer 1 + Layer 2 + Layer 3 |
| Input / Commands | Layer 1 + Layer 2 |
| Sessions | Layer 1 + Layer 2 |
| Configuration | Layer 1 + Layer 2 |
| Architecture / Actors | Layer 1 + Layer 2 |

**Anti-pattern:** Tasks with only functional ACs ("Implement X", "Support Y")
and no test ACs. Every feature must be verifiable by `cargo test`.

## Linter Rules

> The active build script at `crates/runie-core/build.rs` currently enforces
> **1000 lines/file, 120 lines/function, 25 complexity** while the R3
> simplification pass is in progress. The long-term targets remain:

- File max: 500 lines
- Function max: 40 lines, 10 complexity

See `tasks/align-build-rs-lint-thresholds.md`.
