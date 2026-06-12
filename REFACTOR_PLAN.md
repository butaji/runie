# Refactor Plan: Enforce Lint + Reactive Functional Architecture

## Current State
- 20 lint violations (4 files >500 lines, 16 functions >40 lines)
- `AppState` has 28 fields (god object)
- `update.rs` is 623 lines (monolith)
- Update uses `&mut self` imperative style

## Target Architecture

```
┌─────────────┐     Event      ┌─────────────┐     ┌─────────────┐
│ Input Actor │ ──────────────>│             │     │             │
├─────────────┤                │   Event     │     │   Render    │
│ InputState  │                │   Router    │────>│   Actor     │
└─────────────┘                │             │     │             │
                               └──────┬──────┘     └─────────────┘
┌─────────────┐     Event           │
│ Agent Actor │ ───────────────────>│
├─────────────┤                     │
│ AgentState  │                     ▼
└─────────────┘              ┌─────────────┐
                             │   Reducer   │
┌─────────────┐              │   (pure)    │
│  Chat Actor │─────────────>│             │
├─────────────┤              └──────┬──────┘
│ ChatHistory │                     │
└─────────────┘                     ▼
                              ┌────────────┐
                              │  AppState  │
                              │ (composed) │
                              └─────┬──────┘
                                    │ snapshot()
                                    ▼
                              ┌────────────┐
                              │  Snapshot  │
                              │   (MVU)    │
                              └────────────┘
```

## Implementation Steps

### Phase 1: Fix File-Size Violations

| File | Lines | Action |
|------|-------|--------|
| `update.rs` (623) | Split into `update/{mod,input,agent,slash,queue}.rs` | 5 modules |
| `runie-agent/src/lib.rs` (612) | Split into `{agent,tools,turn}.rs` | 3 modules |
| `runie-agent/src/tests.rs` (501) | Split into `tests/{agent,tools,turn}.rs` | 3 modules |
| `runie-term/src/tests/render.rs` (533) | Split into `tests/{render,status,messages}.rs` | 3 modules |

### Phase 2: Split AppState (God Object → Composition)

```rust
// model.rs — composed state
pub struct AppState {
    pub input: InputState,      // 5 fields
    pub chat: ChatHistory,      // 6 fields  
    pub agent: AgentState,      // 10 fields
    pub ui: UiState,            // 7 fields
}

pub struct InputState {
    pub text: String,
    pub suggestions: Option<Vec<String>>,
    pub selected: Option<usize>,
    pub last_query: Option<String>,
}

pub struct ChatHistory {
    pub messages: Vec<ChatMessage>,
    pub scroll: usize,
    pub line_counts: Vec<usize>,
    pub total_lines: usize,
}

pub struct AgentState {
    pub streaming: bool,
    pub turn_active: bool,
    pub inflight: usize,
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub current_request_id: Option<String>,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<Instant>,
    pub thinking_started_at: Option<Instant>,
    pub turn_started_at: Option<Instant>,
}

pub struct UiState {
    pub animation_frame: u32,
    pub all_collapsed: bool,
    pub at_suggestions: Option<Vec<String>>,
    pub at_selected: Option<usize>,
    pub last_at_query: Option<String>,
}
```

### Phase 3: Make Update Pure (Functional Reducer)

Current (imperative):
```rust
impl AppState {
    pub fn update(&mut self, event: Event) {
        match event { ... } // mutates self directly
    }
}
```

Target (functional):
```rust
// Pure reducer: (State, Event) -> State
pub fn reduce(state: AppState, event: Event) -> AppState {
    match event {
        Event::Input(c) => reduce_input(state, c),
        Event::AgentResponse { id, content } => reduce_agent_response(state, id, content),
        ...
    }
}
```

In Rust, full immutability is expensive. Compromise:
- Keep `&mut self` at the top level for performance
- Each reducer is a pure function on a sub-state
- Document: "logically pure, mechanically mutable for zero-copy"

### Phase 4: Fix All Function-Length Violations

| Function | File | Lines | Fix |
|----------|------|-------|-----|
| `render_element` | transform.rs | 60 | Split into per-element helpers |
| `finish_turn` | update.rs | 42 | Extract 6 steps as methods |
| `deliver_queued` | update.rs | 42 | Split steering/follow-up into helpers |
| `messages` | ui.rs | 48 | Extract `build_lines()`, `render_scrollbar()` |
| `run_agent_turn` | agent/lib.rs | 48 | Split into phases |
| `model.rs:132` | provider | 116 | Split model builder |

### Phase 5: Enforce build.rs

- Remove all `unwrap()` / `expect()` from non-test code
- Add `#[deny(clippy::all)]` to lib.rs files
- Make build.rs fail the build on ANY violation (no warnings, only errors)

## Success Criteria

1. `cargo build` passes with zero lint violations
2. `cargo test` still passes all 477+ tests
3. No file >500 lines
4. No function >40 lines  
5. No function >10 complexity
6. `AppState` has ≤10 fields (composed from sub-structs)
7. Update logic is split by domain (input/agent/slash/queue)
