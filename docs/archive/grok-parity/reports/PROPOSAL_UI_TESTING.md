# UI Testing Proposal — Three-Layer Strategy

## Overview

This document defines the testing strategy for Runie's TUI (Terminal User Interface) using a three-layer approach that balances speed, coverage, and maintainability.

## Layer 1: Buffer Assertion Tests (Unit)

**Purpose**: Fast, deterministic tests for individual component rendering.

**Pattern**:
```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn test_top_bar_renders_correctly() {
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let vm = TopBarViewModel {
        repo: "runie".to_string(),
        branch: "main".to_string(),
        ..Default::default()
    };
    
    terminal.draw(|frame| {
        render_top_bar(frame.buffer_mut(), vm, frame.area(), &theme);
    }).unwrap();
    
    let expected = vec![
        "runie/main                                                          0/128k 0% ░░░░░░"
    ];
    terminal.backend().assert_buffer_lines(expected);
}
```

**When to use**:
- Testing format functions (`format_context_window`, `calculate_pct`)
- Testing component render output
- Testing layout calculations
- Testing color/style application

**Pros**: Fast (<10ms), no I/O, precise assertions
**Cons**: Brittle to layout changes, don't test interaction flow

## Layer 2: Flow Integration Tests (State Machine)

**Purpose**: Test user interaction flows through state transitions.

**Pattern**:
```rust
#[test]
fn test_onboarding_flow_provider_to_keyinput() {
    let (mut state, mut palette) = make_state();
    
    // Start at Welcome
    assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
    
    // Press Enter → ProviderSelect
    let cmds = update(&mut state, &mut palette, Msg::OnboardingNext);
    assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::ProviderSelect);
    
    // Select provider
    let cmds = update(&mut state, &mut palette, Msg::OnboardingSelectProvider(0));
    assert!(state.onboarding.as_ref().unwrap().selected_provider.is_some());
    
    // Press Enter → KeyInput
    let cmds = update(&mut state, &mut palette, Msg::OnboardingNext);
    assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::KeyInput);
}
```

**When to use**:
- Testing multi-step user flows (onboarding, permissions)
- Testing state machine transitions
- Testing command palette workflows
- Testing error recovery paths

**Pros**: Tests real user flows, fast, no async needed
**Cons**: Don't test actual rendering, don't test async behavior

## Layer 3: Snapshot Tests (Regression)

**Purpose**: Catch visual regressions across the entire UI.

**Tool**: [`insta`](https://docs.rs/insta) — snapshot testing framework

**Pattern**:
```rust
use insta::assert_debug_snapshot;

#[test]
fn test_full_app_snapshot() {
    let (mut state, mut palette) = make_state_with_onboarding();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Render full app
    terminal.draw(|frame| {
        render_app(frame, &state, &palette, &theme);
    }).unwrap();
    
    // Snapshot the buffer
    assert_debug_snapshot!("app_onboarding_welcome", terminal.backend().buffer());
}
```

**Installation**:
```toml
[dev-dependencies]
insta = { version = "1.39", features = ["yaml", "redactions"] }
```

**Workflow**:
```bash
# Run tests (creates .snap.new files if snapshots differ)
cargo test

# Review changes interactively
cargo insta review

# Accept all pending snapshots
cargo insta accept
```

**Snapshot storage**:
- Snapshots stored in `src/__snapshots__/` or `tests/snapshots/`
- Committed to git for regression tracking
- `.snap.new` files are temporary (add to .gitignore)

**When to use**:
- Testing full-screen rendering
- Testing complex layouts
- Testing multi-component interactions
- Regression testing after refactoring

**Pros**: Comprehensive visual coverage, easy to update, human-reviewable
**Cons**: Slower than unit tests, snapshots need maintenance, large git diffs

## Test Organization

```
crates/runie-tui/src/
├── components/
│   ├── top_bar.rs              # Layer 1 tests inline
│   ├── top_bar_snapshots.rs    # Layer 3 snapshot tests
│   ├── onboarding/
│   │   ├── comprehensive_tests.rs  # Layer 2 flow tests
│   │   └── render_tests.rs         # Layer 1 buffer tests
│   └── command_palette/
│       ├── tests.rs            # Layer 1 unit tests
│       └── tests_flow.rs       # Layer 2 flow tests
├── tui/
│   └── tests/
│       ├── reducer.rs          # Layer 2 state tests
│       ├── render_tests.rs     # Layer 1 render tests
│       └── snapshots/          # Layer 3 insta snapshots
```

## Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p runie-tui

# With snapshots
cargo insta test

# Review snapshot changes
cargo insta review

# CI (fails on unreviewed snapshots)
cargo insta test --check
```

## CI Integration

```yaml
# .github/workflows/ci.yml
- name: Run tests with snapshots
  run: cargo insta test --check
  env:
    INSTA_UPDATE: no
```

## Best Practices

1. **Prefer Layer 1** for new components — fastest feedback
2. **Add Layer 2** for complex flows — state machine coverage
3. **Use Layer 3** sparingly — for regression-prone areas
4. **Name snapshots clearly**: `component_state_description`
5. **Review snapshot diffs** in PRs — they're human-readable
6. **Redact volatile data**: Use `insta::dynamic_redaction()` for timestamps, IDs
7. **Keep snapshots small**: Test individual components, not full app

## Example: Testing Top Bar

```rust
// Layer 1: Unit test
#[test]
fn test_format_context_window() {
    assert_eq!(format_context_window(500), "500");
    assert_eq!(format_context_window(1000), "1k");
    assert_eq!(format_context_window(1_000_000), "1m");
}

// Layer 2: Flow test
#[test]
fn test_top_bar_updates_on_model_change() {
    let (mut state, mut palette) = make_state();
    state.top_bar.model = "gpt-4o".to_string();
    state.top_bar.estimated_tokens = Some(50000);
    
    let vm = TopBarViewModel::from_state(&state);
    assert_eq!(vm.model, "gpt-4o");
    assert_eq!(vm.estimated_tokens, 50000);
}

// Layer 3: Snapshot test
#[test]
fn test_top_bar_snapshot() {
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|frame| {
        render_top_bar(frame.buffer_mut(), make_vm(), frame.area(), &theme);
    }).unwrap();
    
    assert_debug_snapshot!("top_bar_default", terminal.backend().buffer());
}
```

## Migration Plan

1. **Phase 1**: Add `insta` to dev-dependencies, create first snapshot tests
2. **Phase 2**: Convert existing render tests to snapshots where beneficial
3. **Phase 3**: Add snapshot step to CI
4. **Phase 4**: Document snapshot review process for contributors
