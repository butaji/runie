# Runie Simplification Research

## Overview

**Repository**: Runie TUI/CLI for LLM-powered coding agents
**Total Lines**: ~157,614 (production), ~315,228 (with tests)
**Files**: 745 Rust files across 7 crates
**Average Lines/File**: 211.5

## Current Architecture

### Crates
- `runie-core` — Events, AppState, sessions, config, commands, dialog DSL, harness skills
- `runie-agent` — Agent turn loop, tool-call parsing, truncation, subagent runner, built-in tools
- `runie-provider` — LLM provider clients and model catalog
- `runie-tui` — TUI entry, Ratatui rendering, panels/forms, theme, terminal setup
- `runie-cli` — CLI entry, headless/print/server modes
- `runie-testing` — Test fixtures, mock providers, and harness helpers
- `runie-patterns` — Swarm and other execution patterns

---

## Identified Simplification Opportunities

### 1. Event System Code Generation (896 lines → ~200 lines)

**Location**: `crates/runie-core/src/event/generated.rs`

**Problem**: Auto-generated file from Python script with hardcoded variant lists.

**Current State**:
- 896 lines of repetitive match statements
- Hardcoded `UNIT_VARIANTS` and `VARIANTS_WITH_FIELDS` in Python script
- Generated `kind()`, `category()`, `into_intent()`, `is_fact_variant()` functions

**Opportunity**: Derive macro approach

```rust
// Instead of generated match arms, use a derive macro:
#[derive(EventKind)]
pub enum Event { ... }
```

**Potential Reduction**: ~700 lines removed, replaced with ~100 lines of derive macro

**Approach**:
1. Create `runie-derive` crate for `#[derive(EventKind)]`
2. Macro inspects variant names/fields at compile time
3. Generate match tables automatically
4. Single source of truth in `taxonomy.json`

---

### 2. Build Script Complexity (822 lines → ~300 lines)

**Location**: `crates/runie-core/build.rs`

**Problem**: Complex build script with hardcoded pattern lists and exemptions.

**Current Issues**:
- 822 lines of pattern matching code
- 100+ hardcoded exemption paths
- Pattern lists duplicated in Python generator

**Opportunity**: Simplify with configuration-based approach

**Approach**:
1. Move exemptions to `Cargo.toml` metadata
2. Consolidate patterns into a declarative config
3. Use existing Rust linting (Clippy) where possible
4. Consider removing some guardrails if they don't catch real bugs

**Potential Reduction**: ~500 lines

---

### 3. Provider Protocol Unification

**Locations**: 
- `crates/runie-provider/src/openai/protocol.rs` (1202 lines)
- `crates/runie-provider/src/openai/stream.rs` (1027 lines)
- `crates/runie-provider/src/retry.rs` (1068 lines)
- `crates/runie-provider/src/mock.rs` (870 lines)

**Problem**: Provider implementations have similar patterns with code duplication.

**Opportunities**:

#### 3a. Shared Streaming Infrastructure
```
Common: Event buffering, error classification, retry logic
Current: Each provider has its own implementation
```

**Approach**: Extract common streaming utilities:
- Unified SSE parsing traits
- Shared retry policy abstraction
- Common error classification

#### 3b. Mock Provider Simplification
**Current**: 870 lines of hardcoded fixtures
**Opportunity**: Use fixture files + derive-based fixture loading

---

### 4. Test Infrastructure Consolidation

**Stats**:
- 256 test files in `tests/` directories
- 11 `*_tests.rs` files
- Duplicated test utilities across crates

**Opportunities**:

#### 4a. Shared Test Macros
```rust
// Current: Repeated setup code in each test file
// Opportunity: Shared test builder macros
test_case!(list_dir_echo, fixture: "list_dir");
```

#### 4b. Unified Fixtures
- Move fixture loading to shared `runie-testing` crate
- Use `include_str!` or external files consistently
- Eliminate fixture duplication

**Potential Reduction**: ~300-500 lines

---

### 5. UI Component Patterns

**Location**: `crates/runie-tui/src/ui_actor/mod.rs` (976 lines)

**Problem**: Large UI actor with mixed concerns.

**Current State**:
- 976 lines of UI state management
- Mixed input handling and rendering logic
- Bootstrap logic in separate file (967 lines)

**Opportunities**:

#### 5a. Split UI Actor
```
ui_actor/
├── mod.rs          (orchestration, ~300 lines)
├── input.rs        (input handling)
├── render.rs       (rendering logic)
└── state.rs        (state management)
```

#### 5b. Bootstrap Simplification
- Reduce bootstrap configuration complexity
- Use builder pattern more consistently
- Consider declarative UI initialization

**Potential Reduction**: ~300-400 lines

---

### 6. Actor Pattern Boilerplate

**Pattern**: Multiple actors with similar boilerplate:
- ConfigActor
- SessionActor
- TurnActor
- LeaderActor
- ProviderActor
- IoActor

**Current**: Each actor has ~200-400 lines of boilerplate

**Opportunity**: Derive-based actor framework

```rust
// Current pattern (~200 lines per actor):
impl Actor for MyActor {
    type State = MyState;
    fn spawn(...) -> ... { ... }
    fn handle(...) -> ... { ... }
}

// Proposed derive (~50 lines):
#[derive(Actor)]
#[actor(name = "MyActor")]
struct MyActor { ... }
```

**Potential Reduction**: ~500-1000 lines across all actors

**Note**: Existing crates like `theta_macros` provide this pattern. Consider using or inspiration from [theta_macros](https://docs.rs/theta-macros).

---

### 7. Model State Accessors

**Location**: `crates/runie-core/src/model/state/domain_ops.rs` (683 lines)

**Problem**: Extensive accessor methods for AppState.

**Current**: Manual accessor implementations
**Opportunity**: Derive macro for accessor generation

```rust
// Instead of 100+ accessor methods:
#[derive(StateAccessors)]
struct AppState {
    session: SessionState,
    input: InputState,
    // ...
}
```

**Potential Reduction**: ~400 lines

---

### 8. Event Taxonomy JSON (310 lines)

**Location**: `crates/runie-core/src/event/taxonomy.json`

**Problem**: Manual synchronization between taxonomy.json and Event enum.

**Current**: 
- taxonomy.json defines fields/categories
- Python script generates code
- Manual maintenance required

**Opportunity**: Derive-based approach

```rust
#[derive(EventDef)]
enum Event {
    #[event(kind = Fact, category = Agent)]
    Response { id: String, content: String },
    // ...
}
```

**Potential Reduction**: 310 lines (JSON) + ~200 lines (generator) = ~510 lines

---

### 9. Configuration Validation

**Location**: 
- `crates/runie-core/src/config/config_impl.rs` (777 lines)
- `crates/runie-core/src/config/tests/validate_tests.rs` (556 lines)

**Opportunities**:
- Use schema validation libraries
- Reduce manual validation code
- Consolidate test cases

**Potential Reduction**: ~300 lines

---

### 10. Session/State Persistence

**Locations**:
- `crates/runie-core/src/session/replay.rs` (570 lines)
- `crates/runie-core/src/session/store.rs` (554 lines)
- `crates/runie-core/src/session/tree.rs` (569 lines)

**Opportunity**: Unified session persistence abstraction

---

## Recommended Priority Order

### Phase 1: Quick Wins (Low Risk, High Impact)
1. **Event derive macro** — Replace generated.rs (~700 line reduction)
2. **Build script simplification** — Declarative exemptions (~500 lines)
3. **Test macro consolidation** — Shared test utilities (~300 lines)

### Phase 2: Medium Effort
4. **Actor derive macro** — Reduce boilerplate (~500-1000 lines)
5. **State accessor derive** — Replace domain_ops.rs (~400 lines)
6. **Provider streaming unification** — Shared utilities (~300 lines)

### Phase 3: Architectural
7. **UI actor split** — Separate concerns (~300 lines)
8. **Session persistence refactor** — Unified abstraction (~500 lines)

---

## Total Potential Reduction

| Category | Current | Target | Reduction |
|----------|---------|--------|-----------|
| Event system | 896 | 200 | ~700 |
| Build script | 822 | 300 | ~500 |
| Actor boilerplate | ~2500 | ~1500 | ~1000 |
| State accessors | 683 | 300 | ~400 |
| Provider protocols | ~4300 | ~3800 | ~500 |
| UI components | ~2000 | ~1700 | ~300 |
| Session persistence | ~1700 | ~1400 | ~300 |
| Tests/infrastructure | ~5000 | ~4500 | ~500 |
| **Total** | ~17,900 | ~12,700 | **~5,200 lines** |

---

## Research Sources

1. **theta_macros** — Rust actor derive macro crate
   - https://docs.rs/theta-macros
   - Provides `#[actor]` and `ActorArgs` derive macros

2. **Custom Derive Patterns**
   - https://oneuptime.com/blog/post/2026-01-25-custom-derive-macros-rust/view
   - Guide on reducing boilerplate with derive macros

3. **Rust Macros Best Practices**
   - https://microsoft.github.io/RustTraining/rust-patterns-book/ch13.html
   - Use derive macros liberally for boilerplate elimination

4. **Code Simplification Principles**
   - https://optymizer.com/agents/development/code-simplifier/
   - 30% LOC reduction with maintained functionality

---

## Risks and Considerations

1. **Derive Macro Maintenance** — Adds compile-time complexity
2. **Breaking Changes** — Public API changes need careful migration
3. **Testing Burden** — Each simplification needs test coverage
4. **Learning Curve** — Team needs to understand generated code

## Recommendations

1. Start with **Phase 1** (quick wins) to validate approach
2. Create `runie-derive` crate for shared derive macros
3. Maintain backward compatibility during transition
4. Add extensive documentation for generated code
5. Consider incremental migration per crate
