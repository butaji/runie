# Agent Guidelines

Track tasks in tasks/index.json, details per each in tasks/xxx.md


All features, fixes, and improvements must be implemented with coverage of automatic tests that are lightweight and run fast.
No artificail delays in automatic tests!

## Testing Strategy (3 Layers)

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

## Anti-Patterns (Never Do These)

| Don't | Why |
|-------|-----|
| Test through real terminal (tmux, script, etc.) | Flaky, requires TTY |
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

## Linter Rules

- File max: 500 lines
- Function max: 40 lines, 10 complexity
