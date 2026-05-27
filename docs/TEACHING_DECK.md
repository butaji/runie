# Runie Architecture: MVU + Tokio Actors

## Slide 1: The Problem

```
 WITHOUT MVU                          WITH MVU
 ┌─────────────┐                     ┌─────────────┐
 │ Widget A    │──mutates──┐        │   AppState  │◄──single source
 │ Widget B    │──mutates──┼──►??   └──────┬──────┘
 │ Agent Task  │──mutates──┘               │
 │ File Watcher│──mutates──┐               │
 └─────────────┘          State Spaghetti  │ Pure
                                           │ Functions
                                           v
                                    ┌─────────────┐
                                    │   Render    │
                                    └─────────────┘
```

**At 100k+ LOC:**
- 50+ widgets mutating shared state = bugs
- LLM calls blocking UI = frozen TUI
- File ops in render thread = glitches

**Rule:** One `App` struct owns ALL state. Everything else is pure functions or isolated tasks.

---

## Slide 2: Three Pure Functions (TEA/MVU)

```rust
// ╔══════════════════════════════════════════╗
// ║  MODEL  — Single source of truth         ║
// ╚══════════════════════════════════════════╝
struct App {
    messages: Vec<Message>,
    current_model: String,
    mode: Mode,
    // ALL state lives here. Nothing else.
}

// ╔══════════════════════════════════════════╗
// ║  UPDATE — Only place that mutates        ║
// ╚══════════════════════════════════════════╝
fn update(app: &mut App, msg: Msg) -> Option<Cmd> {
    match msg {
        Msg::Submit(text) => {
            app.messages.push(Message::User(text));
            Some(Cmd::SpawnAgent { messages: app.messages.clone() })
        }
        Msg::AgentDone(result) => {
            app.messages.push(Message::Agent(result));
            None
        }
        _ => None,
    }
}

// ╔══════════════════════════════════════════╗
// ║  VIEW   — Pure function, no side effects ║
// ╚══════════════════════════════════════════╝
fn view(app: &App) -> impl Widget {
    // Reads app. Never mutates. No I/O.
    Paragraph::new(format!("{} messages", app.messages.len()))
}
```

**Golden Rules:**
1. `view()` never mutates
2. `update()` never does I/O
3. Only `Cmd` variants trigger side effects

---

## Slide 3: The Event Loop

```rust
// One loop. Everything flows through here.
loop {
    // 1. RECEIVE (from any source)
    let msg = rx.recv().await?;
    
    // 2. UPDATE (only mutation)
    if let Some(cmd) = app.update(msg) {
        // 3. CMD (async side effects)
        match cmd {
            Cmd::SpawnAgent { messages } => {
                let tx = tx.clone();
                tokio::spawn(async move {
                    // Slow work here, NEVER blocks main loop
                    let result = llm_call(messages).await;
                    // Send result BACK through same channel
                    let _ = tx.send(Msg::AgentDone(result)).await;
                });
            }
            Cmd::SaveSettings { config } => {
                tokio::spawn(async move {
                    fs::write("config.toml", config).await.ok();
                });
            }
        }
    }
    
    // 4. RENDER (pure, immediate mode)
    terminal.draw(|f| f.render_widget(app.view(), f.area()))?;
}
```

**Key:** Agents and UI share the **same** `mpsc` channel. Everything is a `Msg`.

---

## Slide 4: Why Actors (Tokio Tasks)

```
┌─────────────────────────────────────────────┐
│           MAIN EVENT LOOP                   │
│  (fast, never blocks, 16ms frame budget)   │
│                                             │
│  loop {                                     │
│    msg = rx.recv().await     ← 1μs         │
│    app.update(msg)           ← 10μs        │
│    terminal.draw(...)        ← 1ms         │
│  }                                          │
└──────────────────┬──────────────────────────┘
                   │ mpsc::channel
                   │
    ┌──────────────┼──────────────┐
    │              │              │
    ▼              ▼              ▼
┌────────┐  ┌──────────┐  ┌──────────┐
│ Agent  │  │ File Ops │  │ Git Stat │
│ Task   │  │ Task     │  │ Task     │
│ (5s)   │  │ (100ms)  │  │ (50ms)   │
└────┬───┘  └────┬─────┘  └────┬─────┘
     │           │             │
     └───────────┴─────────────┘
                 │
                 ▼
         tx.send(Msg::...).await
```

**Without actors:** LLM call blocks UI for 5 seconds = frozen screen
**With actors:** UI stays responsive, agent reports back when done

---

## Slide 5: Scaling to 100k+ LOC

### Split by Domain, Not by Layer

```rust
// DON'T: Split by technical layer (hard to navigate)
struct App {
    // 200 fields from all domains mixed together
}

// DO: Split by domain (teams can own slices)
struct App {
    chat: ChatStore,
    agent: AgentStore,
    files: FileStore,
    ui: UiStore,
}

// Each store owns its state + messages
struct ChatStore {
    messages: Vec<Message>,
    input_buffer: String,
}

enum ChatMsg {
    Submit(String),
    Receive(String),
    Clear,
}

impl ChatStore {
    fn update(&mut self, msg: ChatMsg) -> Option<ChatCmd> {
        match msg {
            ChatMsg::Submit(text) => {
                self.messages.push(Message::User(text));
                Some(ChatCmd::SendToAgent)
            }
            ChatMsg::Receive(text) => {
                self.messages.push(Message::Agent(text));
                None
            }
            ChatMsg::Clear => {
                self.messages.clear();
                self.input_buffer.clear();
                None
            }
        }
    }
}
```

### The Component Trait

```rust
// Every major feature implements this
trait Component {
    type Msg;
    type Cmd;
    
    fn update(&mut self, msg: Self::Msg) -> Option<Self::Cmd>;
    fn view(&self) -> Box<dyn Widget>;
}

// App becomes a dispatcher
impl App {
    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        match msg {
            Msg::Chat(m) => self.chat.update(m).map(Cmd::Chat),
            Msg::Agent(m) => self.agent.update(m).map(Cmd::Agent),
            Msg::File(m) => self.files.update(m).map(Cmd::File),
        }
    }
}
```

---

## Slide 6: Plugin Architecture (Actors)

```rust
// Plugin = Actor with Sender handle
// NEVER touches App state directly

pub trait Plugin: Send {
    fn update(&mut self, msg: PluginMsg) -> Option<PluginCmd>;
    fn view(&self) -> Box<dyn Widget>;
}

// How a plugin talks back to App
struct AgentPlugin {
    tx: mpsc::Sender<Msg>,  // ← handle to main queue
}

impl Plugin for AgentPlugin {
    fn update(&mut self, msg: AgentMsg) -> Option<AgentCmd> {
        match msg {
            AgentMsg::Run(prompt) => {
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    let out = llm_call(prompt).await;
                    // Send result back to SAME queue
                    tx.send(Msg::Agent(AgentMsg::Done(out))).await.ok();
                });
                None
            }
            AgentMsg::Done(out) => {
                self.log.push(out);
                None
            }
        }
    }
}

// App owns plugin states
struct App {
    editor: EditorPlugin,
    agent: AgentPlugin,
    git: GitPlugin,
    tx: mpsc::Sender<Msg>,  // shared with all plugins
}
```

**For dynamic loading (.so/WASM):**
```rust
enum Msg {
    Core(CoreMsg),
    Plugin { id: String, payload: Vec<u8> },
}
// Route by string key. Same flow, small runtime cost.
```

---

## Slide 7: Message Design

```rust
// ONE enum for EVERYTHING
enum Msg {
    // User input
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    
    // Time
    Tick,                    // 80ms animation
    CursorBlink,            // 500ms
    
    // Agent
    AgentStart,
    AgentChunk(String),      // streaming delta
    AgentDone(Result<String, AgentError>),
    AgentToolCall { name: String, args: String },
    
    // File system
    FileLoaded { path: String, content: String },
    FileSaved { path: String },
    
    // Plugin
    Plugin { id: String, payload: Vec<u8> },
    
    // System
    Resize { width: u16, height: u16 },
    FocusGained,
    FocusLost,
}
```

**Rule:** If it can happen, it's a `Msg` variant.

---

## Slide 8: Anti-Patterns (What NOT to Do)

```rust
// ❌ DON'T: Mutate state outside update()
app.messages.push(msg);  // in event loop!

// ✅ DO: Send Msg through update()
let cmd = app.update(Msg::Receive(msg));


// ❌ DON'T: Do I/O in view()
fn view(app: &App) -> impl Widget {
    let files = fs::read_dir(".").unwrap();  // SIDE EFFECT!
    // ...
}

// ✅ DO: Pure function, read only
fn view(app: &App) -> impl Widget {
    Paragraph::new(&app.file_list)  // app.file_list set by update()
}


// ❌ DON'T: Block main loop
let result = llm_call(prompt).await;  // UI frozen for 5s

// ✅ DO: Spawn actor, send result back
tokio::spawn(async move {
    let result = llm_call(prompt).await;
    tx.send(Msg::AgentDone(result)).await.ok();
});


// ❌ DON'T: Clone Settings at spawn (stale data)
let settings = settings.clone();
tokio::spawn(async move {
    create_provider(&settings);  // stale if user changes model!
});

// ✅ DO: Read fresh from AppState at spawn time
let fresh = FreshSettings::from_state(&app.state);
tokio::spawn(async move {
    create_provider(fresh.provider, fresh.model, fresh.api_key);
});
```

---

## Slide 9: Testing Strategy

```rust
// Pure functions = easy testing

#[test]
fn test_submit_creates_user_message() {
    let mut app = App::default();
    
    let cmd = app.update(Msg::Submit("hello".into()));
    
    assert_eq!(app.messages.len(), 1);
    assert!(matches!(&app.messages[0], Message::User(s) if s == "hello"));
    assert!(matches!(cmd, Some(Cmd::SpawnAgent { .. })));
}

#[test]
fn test_view_shows_message_count() {
    let mut app = App::default();
    app.messages.push(Message::User("hi".into()));
    
    let widget = app.view();
    // Inspect widget buffer...
}

#[test]
fn test_full_flow() {
    let mut app = App::default();
    
    // User submits
    app.update(Msg::Submit("question".into()));
    assert_eq!(app.messages.len(), 1);
    
    // Agent responds
    app.update(Msg::AgentChunk("answer".into()));
    assert_eq!(app.messages.len(), 2);
    
    // Agent done
    app.update(Msg::AgentDone(Ok("final".into())));
    assert!(!app.agent_running);
}
```

**No mocking needed.** No async runtime. Just pure functions.

---

## Slide 10: Summary

```
┌─────────────────────────────────────────────┐
│              ARCHITECTURE                   │
├─────────────────────────────────────────────┤
│                                             │
│  External Events                            │
│       │                                     │
│       ▼ (mpsc channel)                      │
│  ┌─────────────┐                            │
│  │    Msg      │  ← ONE enum for all        │
│  └──────┬──────┘                            │
│         │                                   │
│         ▼                                   │
│  ┌─────────────┐     ┌─────────────┐       │
│  │   update()  │────►│   AppState  │       │
│  │  (mutates)  │     │  (single    │       │
│  └──────┬──────┘     │   source)   │       │
│         │            └──────┬──────┘       │
│    ┌────┴────┐              │              │
│    │         │              │              │
│    ▼         ▼              ▼              │
│  ┌────┐   ┌────┐     ┌─────────────┐      │
│  │Cmd │   │Cmd │     │    view()   │      │
│  │Spawn│   │Save│     │  (pure fn)  │      │
│  └──┬─┘   └──┬─┘     └──────┬──────┘      │
│     │        │              │              │
│     ▼        ▼              ▼              │
│  ┌──────┐ ┌──────┐   ┌─────────────┐      │
│  │Agent │ │File  │   │   Ratatui   │      │
│  │Task  │ │Task  │   │   Render    │      │
│  └──┬───┘ └──┬───┘   └─────────────┘      │
│     │        │                             │
│     └────────┴──► Msg back to loop         │
│                                             │
└─────────────────────────────────────────────┘
```

**Core Principles:**
1. **Single Source:** `AppState` owns everything
2. **Pure Update:** `update()` is the only mutator
3. **Pure View:** `view()` reads, never writes
4. **Async Actors:** Tokio tasks for slow work
5. **Unified Channel:** One `Msg` enum, one `mpsc` channel
6. **Split by Domain:** Stores for each feature area

**Result:** Predictable state. Testable logic. Responsive UI. Scalable to 100k+ LOC.
