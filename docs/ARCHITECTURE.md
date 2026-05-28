# Runie Architecture

## Core Principles

### 1. Everything is an Actor

Every concurrent operation is a Tokio task. No blocking calls in the main loop.

```rust
// Terminal reader actor
tokio::task::spawn_blocking(move || {
    while !cancel.is_cancelled() {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(event) = crossterm::event::read() {
                let msgs = convert_event_to_msg(event);
                for msg in msgs { msg_tx.try_send(msg).ok(); }
            }
        }
    }
});

// Agent actor
tokio::spawn(async move {
    run_agent_loop(messages, config, provider, msg_tx).await
});

// Model fetch actor  
tokio::spawn(async move {
    let models = fetcher.fetch_models(api_key).await;
    msg_tx.send(Msg::ModelsFetched(models)).await.ok();
});
```

Actors communicate exclusively through messages. No shared state, no locks, no callbacks.

### 2. Global Event Stream

**One channel. One enum. No exceptions.**

```rust
// Single mpsc channel for ALL events
let (msg_tx, msg_rx) = mpsc::channel::<Msg>(100);

// Every state change is a message
enum Msg {
    // User input
    Key(KeyEvent),
    Paste(String),
    Resize(u16, u16),
    
    // Agent events
    AgentEvent(AgentEvent),
    
    // HTTP results
    ModelsFetched(Vec<ModelInfo>),
    ModelsFetchFailed(String),
    
    // File system
    FileRead { path: String, content: String },
    
    // UI commands
    SetMode(TuiMode),
    OpenOverlay(Overlay),
    
    // Timer
    Tick,
    CursorBlink,
}
```

**No direct state mutations.** All state changes go through `update(state, msg)`:

```rust
// WRONG: Direct mutation
state.top_bar.repo = git_info.repo;

// RIGHT: Through event stream
msg_tx.send(Msg::SetGitInfo { repo, branch, path }).await?;
```

**No side channels.** Permission decisions, agent events, HTTP responses — all through the same `Msg` enum.

### 3. MVU / TUI

```
Event → Msg → update() → (new State, Cmds) → render() → Terminal
```

**Model** — `AppState` is the single source of truth:
```rust
pub struct AppState {
    pub chat: ChatStore,
    pub agent: AgentStore,
    pub ui: UiStore,
}
```

**Update** — Pure function, only place that mutates state:
```rust
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::Chat(m) => chat::update(&mut state.chat, m),
        Msg::Agent(m) => agent::update(&mut state.agent, m),
        Msg::Ui(m) => ui::update(&mut state.ui, m),
    }
}
```

**View** — Pure render, no side effects:
```rust
terminal.draw(|frame| {
    render_normal_mode(frame.buffer_mut(), &state, &vms);
})?;
```

### 4. Purist Ratatui

Render directly into `Frame`. No manual buffer management. No dirty flags. Ratatui handles diffing internally.

```rust
// WRONG: Manual buffer
let mut buf = Buffer::empty(area);
render_widgets(&mut buf);
terminal.draw(|frame| {
    copy_buffer_to_frame(&buf, frame);
})?;

// RIGHT: Direct render
terminal.draw(|frame| {
    frame.render_widget(top_bar, top_area);
    frame.render_widget(message_list, content_area);
    frame.render_widget(input_bar, input_area);
})?;
```

**No throttling.** Render on every state change. Ratatui's buffer diffing is fast enough.

### 5. Purist Rust

**Type-safe events.** Every event is a strongly-typed enum variant. No strings, no maps, no dynamic typing.

**Exhaustive matching.** The compiler ensures every `Msg` variant is handled:
```rust
match msg {
    Msg::Key(k) => handle_key(k),
    Msg::Paste(t) => handle_paste(t),
    // Compiler error if you forget a variant
}
```

**Move semantics.** Data ownership is clear. No accidental clones, no shared mutable state.

**Zero-cost abstractions.** The event stream compiles down to simple enum dispatch. No runtime overhead.

## Static Model Registry

Following [Pi](https://github.com/pi)'s approach, we use a **static, hardcoded model registry** instead of runtime API fetching.

**Why static?**
- No network dependency during onboarding
- Instant model list (no loading states)
- Predictable, version-controlled model data
- Works offline

**Trade-off:** Models must be updated manually when providers add new ones. The registry is generated from Pi's `models.generated.ts` which itself is auto-generated from `models.dev` API at build time.

```rust
// crates/runie-ai/src/model_fetcher.rs
pub fn get_provider_models(provider: &str) -> Option<Vec<ModelInfo>> {
    let mut registry: HashMap<&str, Vec<ModelInfo>> = HashMap::new();
    
    registry.insert("openai", vec![
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4.1".to_string(), name: "GPT-4.1".to_string() },
        // ... 40+ more models
    ]);
    
    registry.insert("anthropic", vec![
        ModelInfo { id: "claude-sonnet-4-6".to_string(), name: "Claude Sonnet 4.6".to_string() },
        // ... 20+ more models
    ]);
    
    // 20+ providers, 500+ models total
    registry.get(provider).cloned()
}
```

The `ModelFetcher` trait still exists for API compatibility, but `fetch_models()` now returns the static list immediately — no HTTP calls, no async runtime needed.

## Event Flow Example

```
User presses Enter in onboarding
    ↓
Terminal reader captures KeyEvent(Enter)
    ↓
Converts to Msg::OnboardingNext
    ↓
Sends to msg_tx
    ↓
Main loop receives via msg_rx.recv()
    ↓
update(&mut state, Msg::OnboardingNext)
    ↓
State changes: step = KeyInput, is_fetching_models = true
    ↓
Returns Cmd::FetchModels { provider: "minimax", api_key }
    ↓
Spawns fetch actor
    ↓
Renders immediately: shows "loading models..."
    ↓
... 0ms later (static lookup) ...
    ↓
Fetch actor completes, sends Msg::ModelsFetched(models)
    ↓
Main loop receives
    ↓
update(&mut state, Msg::ModelsFetched(models))
    ↓
State changes: models = models, step = ModelSelect
    ↓
Renders immediately: shows model picker
```

Every step is a message. Every state change is visible. No hidden mutations.

## File Structure

```
crates/
  runie-core/        # Msg, Cmd, AgentEvent, shared types
  runie-agent/       # Agent loop, tool execution
  runie-ai/          # Provider abstractions, static model registry
  runie-tui/         # State, update, render, components
  runie-cli/         # Main loop, actor spawning, event routing
```

## Testing

Pure functions = easy testing:
```rust
#[test]
fn test_submit_creates_user_message() {
    let mut state = AppState::default();
    let cmds = update(&mut state, Msg::Chat(ChatMsg::Submit));
    
    assert_eq!(state.chat.messages.len(), 1);
    assert!(matches!(&state.chat.messages[0], MessageItem::User { .. }));
}
```

No async runtime. No mocks. Just pure functions.

## Provider Support

| Provider | Source | Models |
|----------|--------|--------|
| OpenAI | Pi/static | 40+ |
| Anthropic | Pi/static | 20+ |
| Groq | Pi/static | 15+ |
| Together | Pi/static | 20+ |
| xAI | Pi/static | 7+ |
| Mistral | Pi/static | 25+ |
| DeepSeek | Pi/static | 2+ |
| OpenRouter | Pi/static | 150+ |
| MiniMax | Pi/static | 2+ |
| HuggingFace | Pi/static | 15+ |
| Z.ai | Pi/static | 5+ |
| Google | Static | 9+ |
| Ollama | Static | 10+ |
| Azure | Static | 4 |
| Cohere | Static | 4 |
| Perplexity | Static | 4 |
| Moonshot | Static | 3 |
| Hyperbolic | Static | 4 |

Total: 500+ models across 20+ providers.
