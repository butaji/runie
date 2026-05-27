# Runie Unified Architecture Design

## Philosophy

Runie combines the **MVU (Model-View-Update)** pattern with **TEA (The Elm Architecture)** principles for state management, while leveraging **Tokio actors** for async side effects. The result: a purely functional UI state machine with pragmatic side-effect handling.

**Core principle**: `AppState` is the **sole mutable state**. `update()` is the **only mutator**. `render()` is **pure**. Actors communicate via a **unified channel**, not return types.

---

## State (Model)

```rust
pub struct AppState {
    pub chat: ChatStore,      // messages, input, scroll
    pub agent: AgentStore,    // config, steering_queue, active_run
    pub ui: UiStore,          // mode, focused, overlays, theme
}
```

### Domain Stores Are Self-Contained

Each store encapsulates its domain completely:

```rust
// ChatStore - owns chat domain
pub struct ChatStore {
    pub messages: Vec<MessageItem>,
    pub input: String,
    pub scroll_offset: usize,
    pub pending_submit: Option<String>,
}

// AgentStore - owns agent domain  
pub struct AgentStore {
    pub config: AgentConfig,
    pub steering_queue: VecDeque<String>,   // user steering commands
    pub active_run: Option<ActiveRun>,      // current agent execution
    pub tool_history: Vec<ToolCallRecord>, // for doomscroll detection
    pub cancellation_token: CancellationToken,
}

// UiStore - owns UI domain
pub struct UiStore {
    pub mode: UiMode,
    pub focused: FocusedComponent,
    pub overlays: Vec<Overlay>,
    pub theme: ThemeWrapper,
    pub terminal_size: (u16, u16),
}
```

**Rule**: No store reaches into another store. `ChatStore` never touches `AgentStore`. `UiStore` never mutates `ChatStore`.

---

## Messages (Update Inputs)

### Unified Msg Enum

All inputs flow through ONE enum:

```rust
pub enum Msg {
    Chat(ChatMsg),
    Agent(AgentMsg),
    Ui(UiMsg),
    System(SystemMsg),
}
```

### Domain Message Enums

```rust
// Chat domain - user input and chat state changes
pub enum ChatMsg {
    Submit,                              // send message
    InputKey(KeyEvent),                   // keyboard in input
    Receive(MessageItem),                 // message received (from agent)
    Scroll(i32),                          // scroll offset delta
    Clear,                                // clear chat history
}

// Agent domain - agent execution and tools
pub enum AgentMsg {
    Run(String),                          // start agent with prompt
    Steer(String),                        // add steering command
    Event(AgentEvent),                    // event from agent actor
    ToolResult(ToolResult),               // result from tool execution
    Interrupt,                            // cancel current run
    DoomscrollDetected { tool_name: String, count: usize },
}

// UI domain - mode, focus, overlays
pub enum UiMsg {
    SetMode(UiMode),
    Focus(FocusedComponent),
    OpenOverlay(Overlay),
    CloseOverlay,
    ToggleSidebar,
    Resize(u16, u16),
}

// System domain - global inputs
pub enum SystemMsg {
    Tick,                                 // animation frame
    Key(KeyEvent),                        // raw key event
    Paste(String),                        // clipboard paste
    Resize(u16, u16),
}
```

### Message Routing

Input events are converted to `Msg` before update:

```rust
// events.rs
pub fn event_to_msg(event: Event, state: &AppState) -> Vec<Msg> {
    match event {
        Event::Key(key) => {
            // Global shortcuts first
            if let Some(msg) = try_global_hotkey(&key, state) {
                return vec![msg];
            }
            // Route to focused domain
            route_to_focused(&key, state)
        }
        Event::Paste(text) => vec![Msg::System(SystemMsg::Paste(text))],
        Event::Resize(w, h) => vec![Msg::System(SystemMsg::Resize(w, h))],
        _ => vec![],
    }
}
```

---

## Update (The Reducer)

### Main Dispatch

```rust
pub fn update(state: &mut AppState, msg: Msg, now: Instant) -> Vec<Cmd> {
    match msg {
        Msg::Chat(m) => chat::update(&mut state.chat, m, now),
        Msg::Agent(m) => agent::update(&mut state.agent, m, now),
        Msg::Ui(m) => ui::update(&mut state.ui, m, now),
        Msg::System(m) => system::update(state, m, now),
    }
}
```

### Domain Update Functions

Each domain has its own update function returning domain-specific `Cmd`:

```rust
// chat.rs
pub fn update(chat: &mut ChatStore, msg: ChatMsg, now: Instant) -> Vec<ChatCmd> {
    match msg {
        ChatMsg::Submit => vec![ChatCmd::SpawnAgent],
        ChatMsg::InputKey(key) => handle_input(chat, key),
        ChatMsg::Receive(item) => { chat.messages.push(item); vec![] }
        ChatMsg::Scroll(delta) => { chat.scroll_offset = (chat.scroll_offset as i32 + delta) as usize; vec![] }
        ChatMsg::Clear => { chat.messages.clear(); vec![] }
    }
}

// agent.rs
pub fn update(agent: &mut AgentStore, msg: AgentMsg, now: Instant) -> Vec<AgentCmd> {
    match msg {
        AgentMsg::Run(prompt) => {
            agent.active_run = Some(ActiveRun::new(prompt));
            vec![AgentCmd::SpawnAgent { prompt }]
        }
        AgentMsg::Steer(text) => {
            agent.steering_queue.push_back(text);
            vec![]
        }
        AgentMsg::Event(event) => handle_agent_event(agent, event),
        AgentMsg::Interrupt => {
            agent.cancellation_token.cancel();
            agent.active_run = None;
            vec![]
        }
        AgentMsg::DoomscrollDetected { .. } => {
            // Show confirmation overlay
            vec![AgentCmd::ShowDoomscrollConfirm]
        }
        _ => vec![],
    }
}

// ui.rs
pub fn update(ui: &mut UiStore, msg: UiMsg, _now: Instant) -> Vec<UiCmd> {
    match msg {
        UiMsg::OpenOverlay(o) => { ui.overlays.push(o); vec![] }
        UiMsg::CloseOverlay => { ui.overlays.pop(); vec![] }
        UiMsg::Focus(f) => { ui.focused = f; vec![] }
        UiMsg::SetMode(m) => { ui.mode = m; vec![] }
        _ => vec![],
    }
}
```

### Cmd Mapping

Domain `Cmd` types are mapped to global `Cmd`:

```rust
pub enum Cmd {
    // From chat domain
    SpawnAgent { messages: Vec<AgentMessage> },
    
    // From agent domain
    InterruptAgent,
    ShowDoomscrollConfirm,
    
    // From ui domain
    SaveSettings(Settings),
    FetchModels(String),
    GitStatus,
}
```

---

## Commands (Side Effects)

`Cmd` represents side effects to be executed by the runtime. The main loop executes commands:

```rust
impl Tui {
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        let cmds = update(&mut self.state, msg, Instant::now());
        // Commands are returned, not executed here
        // Runtime executes them and sends results back as Msg
        cmds
    }
    
    pub fn execute(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::SpawnAgent { messages } => self.spawn_agent(messages),
            Cmd::InterruptAgent => self.interrupt_agent(),
            Cmd::SaveSettings(s) => self.save_settings(s),
            Cmd::FetchModels(provider) => self.fetch_models(provider),
            Cmd::GitStatus => self.update_git_status(),
        }
    }
}
```

### Actor Spawning

Agents run as Tokio tasks, communicating via channels:

```rust
pub fn spawn_agent(&mut self, messages: Vec<AgentMessage>) {
    let (tx, rx) = mpsc::channel::<Msg>(32);
    
    // Store sender for interruption
    self.agent_task_tx = Some(tx.clone());
    
    tokio::spawn(async move {
        let agent = Agent::new(messages, tx);
        agent.run().await;
    });
    
    // Spawn message receiver
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Convert agent events to Msg and send to main loop
            self.update(Msg::Agent(msg));
        }
    });
}
```

---

## Components (Pure Rendering)

Components are **pure rendering functions**. No state, no input handling:

```rust
pub trait Component {
    type ViewModel: Clone;
    fn render(vm: &Self::ViewModel, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper);
}
```

### ViewModel Pattern

State is transformed into view models before rendering:

```rust
pub struct ChatViewModel {
    pub messages: Vec<MessageVm>,
    pub input_visible: bool,
    pub scroll_offset: usize,
}

pub fn render_chat(chat: &ChatStore, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let vm = ChatViewModel::from(chat);
    MessageList.render(&vm, area, buf, theme);
}
```

### Component Hierarchy

```
┌─────────────────────────────────────────┐
│ App (root)                              │
 ├─ TopBar                                │
 ├─ ChatPanel                             │
 │   ├─ MessageList                       │
 │   │   ├─ UserMessage                  │
 │   │   ├─ AssistantMessage             │
 │   │   ├─ ToolCall                    │
 │   │   └─ ErrorMessage                │
 │   └─ InputBar                         │
 ├─ Sidebar                               │
 │   └─ AgentList                        │
 ├─ StatusBar                            │
 └─ OverlayStack                         │
     ├─ PermissionModal                  │
     ├─ CommandPalette                   │
     └─ DiffViewer                       │
└─────────────────────────────────────────┘
```

---

## Key Features

### 1. Steering Queue

User steering commands queue up and are drained at turn boundaries:

```rust
pub struct AgentStore {
    pub steering_queue: VecDeque<String>,
    // ...
}

// User sends steering command
AgentMsg::Steer(text) => {
    agent.steering_queue.push_back(text);
    vec![]
}

// Agent actor drains at turn start
async fn run_turn(&mut self) -> Result<AgentEvent> {
    // Drain steering queue at turn boundary
    while let Some(steering) = self.state.steering_queue.pop_front() {
        self.prompt.push_str(&format!("\n[Steering: {}]", steering));
    }
    // ... continue with turn
}
```

### 2. Cooperative Cancellation

`CancellationToken` passed to agent actor, checked at natural boundaries:

```rust
pub struct AgentStore {
    pub cancellation_token: CancellationToken,
}

AgentMsg::Interrupt => {
    agent.cancellation_token.cancel();
    vec![]
}

// In agent actor loop
async fn run(&mut self) {
    let token = self.cancellation_token.clone();
    
    // Check at turn start
    token.checkself.cancellation().await;
    
    // ... run turn
    
    // Check between tool calls
    for tool_call in tool_calls {
        token.checkself.cancellation().await;
        self.execute_tool(tool_call).await;
    }
}
```

### 3. Parallel Tool Execution

Independent tool calls run concurrently via `tokio::join!`:

```rust
async fn execute_tools(&self, tools: Vec<ToolCall>) -> Vec<ToolResult> {
    // Tools are independent - run in parallel
    let futures = tools.into_iter().map(|t| self.execute_single_tool(t));
    let results = futures::join_all(futures).await;
    results
}
```

Results sent back as individual `Msg::Agent(ToolResult)` messages.

### 4. Doomscroll Detection

Track last N tool calls; if identical, signal confirmation:

```rust
pub struct AgentStore {
    pub tool_history: Vec<ToolCallRecord>,
}

const DOOMSCROLL_THRESHOLD: usize = 3;

AgentMsg::Event(AgentEvent::ToolExecutionEnd { result }) => {
    let record = ToolCallRecord {
        name: result.tool_name.clone(),
        args_hash: hash(&result.args),
    };
    
    agent.tool_history.push(record);
    
    // Check for doomscroll pattern
    if detect_doomscroll(&agent.tool_history) {
        return vec![AgentMsg::DoomscrollDetected {
            tool_name: result.tool_name,
            count: DOOMSCROLL_THRESHOLD,
        }];
    }
    vec![]
}

fn detect_doomscroll(history: &[ToolCallRecord]) -> bool {
    if history.len() < DOOMSCROLL_THRESHOLD {
        return false;
    }
    let last = &history[history.len() - DOOMSCROLL_THRESHOLD..];
    last.iter().all(|r| r.name == last[0].name && r.args_hash == last[0].args_hash)
}
```

### 5. Overlay Stack

`UiStore.overlays: Vec<Overlay>` - Esc pops top overlay:

```rust
pub enum UiMsg {
    OpenOverlay(Overlay) => { ui.overlays.push(overlay); vec![] }
    CloseOverlay => { ui.overlays.pop(); vec![] }
}

// In render
fn render_overlays(ui: &UiStore, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    // Dim background
    for overlay in &ui.overlays {
        render_overlay(overlay, area, buf, theme);
    }
}
```

### 6. Focused Components

Input routed to focused domain first:

```rust
fn route_input(key: KeyEvent, state: &AppState) -> Msg {
    match state.ui.focused {
        FocusedComponent::Chat => chat::handle_key(key),
        FocusedComponent::CommandPalette => palette::handle_key(key),
        FocusedComponent::Overlay => overlay::handle_key(key),
    }
}
```

### 7. Model Registry

Runtime metadata from provider APIs:

```rust
pub struct AppState {
    pub models: HashMap<String, ModelInfo>,
}

pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub context_window: usize,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
}
```

---

## What NOT to Include

### NO Agent Struct in State

Breaking MVU:

```rust
// BAD - Agent in state
pub struct AppState {
    pub agent: Agent,  // NO!
}

// GOOD - AgentStore in state
pub struct AppState {
    pub agent: AgentStore,  // YES
}
```

The `Agent` struct is an actor, not state. State lives in `AgentStore`.

### NO EventStream Return Type

Breaking unified channel:

```rust
// BAD - EventStream breaks unified channel
pub fn update(state: &mut AppState, msg: Msg) -> (Vec<Cmd>, EventStream) {
    // NO!
}

// GOOD - Everything through Msg
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    // YES!
}
```

### NO External Process Hooks

Too complex for MVP. If needed later:

```rust
// Deferred consideration - not in MVP
// pub enum Cmd {
//     ExecuteExternal { program: String, args: Vec<String> },
// }
```

### NO Client-Server Split

Single binary. All in-process.

### NO Effect Framework

Rust doesn't need Effect. Use `Cmd` + `tokio::spawn`.

---

## File Structure

```
crates/runie-tui/src/
  tui/
    state.rs              # AppState + domain stores
    state/
      chat.rs             # ChatStore + ChatMsg
      agent.rs            # AgentStore + AgentMsg
      ui.rs               # UiStore + UiMsg
    update.rs             # Main dispatch
    update/
      chat.rs             # Chat update logic
      agent.rs            # Agent update logic  
      ui.rs               # UI update logic
      system.rs           # System update logic
    events.rs             # Input routing
    view_models.rs        # Domain view models
    render.rs             # Main render orchestration
  components/
    component.rs          # Pure Component trait
    message_list.rs       # Chat messages
    input_bar.rs          # User input
    sidebar.rs            # Agent list
    overlays/
      mod.rs              # Overlay stack
      permission.rs       # Permission modal
      command_palette.rs  # Command palette
      diff_viewer.rs      # Diff viewer
```

---

## Data Flow

```
┌─────────────┐
│   Input     │ (keyboard, mouse, paste, resize)
└──────┬──────┘
       │ event
       ▼
┌─────────────┐
│ events.rs   │ event_to_msg()
└──────┬──────┘
       │ Msg
       ▼
┌─────────────┐
│ update()    │ Domain dispatch
└──────┬──────┘
       │ Vec<Cmd>
       ▼
┌─────────────┐
│  Runtime    │ Execute commands, spawn actors
└──────┬──────┘
       │ Msg (from actors)
       ▼
┌─────────────┐
│ update()    │ Process events
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ AppState    │ (sole mutable state)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ ViewModels  │ Transform state to VM
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Components  │ Pure render (Rect, Buffer)
└─────────────┘
```

---

## Example: Full Message Flow

### User Submits Message

```
1. KeyEvent(Enter)
   → events::event_to_msg()
   → Msg::Chat(ChatMsg::Submit)

2. update(state, Msg::Chat(ChatMsg::Submit))
   → chat::update()
   → ChatCmd::SpawnAgent

3. Tui::execute(Cmd::SpawnAgent { messages })
   → spawn_agent_task()
   → tokio::spawn(agent_loop(messages, tx))

4. Agent loop runs, sends events:
   → AgentEvent::MessageStart { ... }
   → Msg::Agent(AgentMsg::Event(AgentEvent::MessageStart))
   
5. update(state, Msg::Agent(AgentMsg::Event(...)))
   → agent::update()
   → Updates ChatStore.messages

6. render() called
   → ViewModels::from(ChatStore)
   → MessageList.render(vm, area, buf)
```

### User Interrupts Agent

```
1. Ctrl+C
   → events::event_to_msg()
   → Msg::System(SystemMsg::Key(KeyEvent { code: CtrlC }))

2. try_global_hotkey()
   → Some(Msg::Agent(AgentMsg::Interrupt))

3. update(state, Msg::Agent(AgentMsg::Interrupt))
   → agent::update()
   → AgentStore.cancellation_token.cancel()
   → Cmd::InterruptAgent

4. Tui::execute(Cmd::InterruptAgent)
   → send interrupt to agent task
   → agent task checks token, exits gracefully
```

---

## Type Safety

All message variants are exhaustive:

```rust
impl Msg {
    pub fn chat(msg: ChatMsg) -> Self { Msg::Chat(msg) }
    pub fn agent(msg: AgentMsg) -> Self { Msg::Agent(msg) }
    pub fn ui(msg: UiMsg) -> Self { Msg::Ui(msg) }
    pub fn system(msg: SystemMsg) -> Self { Msg::System(msg) }
}
```

Domain update functions are isolated:

```rust
// chat.rs - only knows about ChatStore and ChatMsg
pub fn update(chat: &mut ChatStore, msg: ChatMsg, now: Instant) -> Vec<ChatCmd>

// agent.rs - only knows about AgentStore and AgentMsg  
pub fn update(agent: &mut AgentStore, msg: AgentMsg, now: Instant) -> Vec<AgentCmd>

// ui.rs - only knows about UiStore and UiMsg
pub fn update(ui: &mut UiStore, msg: UiMsg, now: Instant) -> Vec<UiCmd>
```

This makes it impossible for chat logic to accidentally mutate agent state.

---

## Testing Strategy

### Unit Test Domain Updates

```rust
#[test]
fn test_chat_scroll() {
    let mut chat = ChatStore::default();
    chat.messages = vec![/* 10 messages */];
    
    update(&mut chat, ChatMsg::Scroll(5), Instant::now());
    
    assert_eq!(chat.scroll_offset, 5);
}

#[test]
fn test_steering_queue() {
    let mut agent = AgentStore::default();
    
    update(&mut agent, AgentMsg::Steer("focus on tests".into()), Instant::now());
    update(&mut agent, AgentMsg::Steer("use mock mode".into()), Instant::now());
    
    assert_eq!(agent.steering_queue.len(), 2);
}

#[test]
fn test_doomscroll_detection() {
    let mut agent = AgentStore::default();
    
    // 3 identical tool calls
    for _ in 0..3 {
        update(&mut agent, AgentMsg::Event(AgentEvent::ToolExecutionEnd {
            result: ToolResult {
                tool_name: "read_file".into(),
                args: "{}".into(),
                content: vec![],
                is_error: false,
            }
        }), Instant::now());
    }
    
    // Should trigger doomscroll confirmation
    assert!(matches!(
        agent_update_result.last(),
        Some(AgentCmd::ShowDoomscrollConfirm)
    ));
}
```

### Integration Test Message Flow

```rust
#[test]
fn test_submit_message_flow() {
    let mut state = AppState::default();
    
    // Submit
    let cmds = update(&mut state, Msg::Chat(ChatMsg::Submit), Instant::now());
    
    // Should spawn agent
    assert!(matches!(cmds.first(), Some(Cmd::SpawnAgent { .. })));
}
```
