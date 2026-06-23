# Agent Guidelines

Track tasks in tasks/index.json, details per each in tasks/xxx.md

All features, fixes, and improvements must be implemented with coverage of automatic tests that are lightweight and run fast: unit and e2e.
No artificail delays in automatic tests!

Each implementation must be live tested to make sure everything is working as expected.

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
    let provider = DynProvider::from_provider(
        Box::new(replay_minimax_m3_fixture()) as Box<dyn Provider>
    );
    let skills: Vec<Box<dyn HarnessSkill>> = vec![
        Box::new(MockToolSkill::new(hashmap! {
            "list_dir" => "README.md\nCargo.toml\nsrc/",
            "read_file" => "# Runie\nTerminal coding agent harness.",
        })),
    ];
    let events = run_agent_turn_with_skills(provider, "list files and read README", skills).await;
    assert!(events.iter().any(|e| matches!(e, AgentEvent::TurnComplete)));
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

**STRICT ENFORCEMENT**

The build script at `crates/runie-core/build.rs` enforces these limits:

| Metric | Limit | Scope |
|--------|-------|-------|
| File lines | **500** | Every `.rs` file |
| Function lines | **40** | Production code only |
| Complexity | **10** | Production code only |

File-length limits apply to every source file without exception. Function-length and complexity limits apply to production code only; test functions and files under `tests/` directories are exempt so tests can remain comprehensive.

Complexity is an approximate heuristic (`crates/runie-core/build.rs`) that counts
`if`, `else if`, `match`, `while`, `for`, `loop`, `break`, `continue`, `return`,
`&&`, `||`, and `?` tokens. It is intentionally lightweight and does not parse
Rust syntax, so it may miss nested closures, `try` blocks, match guards, and
similar constructs. It is used as a coarse guardrail, not a precise metric.

Any production-code violation fails `cargo build`. There are no allow-lists.

Violations are detected automatically by `cargo build`; always run `cargo build --workspace` before claiming the codebase is clean.

**Breaking the rules is not acceptable.** If your change introduces a violation, you must fix it before committing.
