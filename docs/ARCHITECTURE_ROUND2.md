# Runie TUI Architecture Round 2

> **DRAFT — NOT IMPLEMENTED.** This document describes a proposed redesign of
> the runie architecture (sub-states, single `Msg` enum, etc.). It is kept here
> for reference but the code does not yet match this design. See
> [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the actually-implemented
> architecture.

## Overview

Round 2 architecture replaces the current monolithic AppState + ad-hoc update pattern with a clean **pipe-based architecture** where data flows through typed channels, actors own I/O scopes, and builders produce all view models.

## Current Problems

1. **AppState has 33 fields** — tightly coupled, hard to test, impossible to reason about
2. **Actor framework is dead code** — uses JSON serialization for type erasure (line 1009 in framework.rs: `serde_json::to_string`)
3. **Builders are disconnected** — exist in tests only, production uses `ViewModels::from_render_state` with inline functions
4. **No clear boundaries** — InputActor, TimerActor, and AgentActor all exist but aren't integrated into a pipe system

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           APPLICATION                                    │
│                                                                          │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐            │
│  │  InputPipe   │────▶│  StatePipe   │────▶│ ViewModelPipe│            │
│  │              │     │              │     │              │            │
│  │ crossterm   │     │ Msg ──▶State │     │ State ──▶VM  │            │
│  │ events      │     │ (reducer)    │     │ (builders)   │            │
│  └──────────────┘     └──────────────┘     └──────────────┘            │
│         ▲                    ▲                    │                       │
│         │                    │                    ▼                       │
│  ┌──────┴──────┐     ┌──────┴──────┐     ┌──────────────┐            │
│  │ InputActor  │     │  RenderPipe │     │   RenderPipe │            │
│  │ (I/O scope) │     │             │◀────│              │            │
│  └─────────────┘     │ ViewModels  │     │  Terminal    │            │
│         │            │ ──▶Frame    │     │  draw()      │            │
│         │            └──────────────┘     └──────────────┘            │
│         │                   ▲                                         │
│  ┌──────┴──────┐          │                                         │
│  │ TimerActor  │──────────┘                                         │
│  │ (Tick/Cursor│                                                        │
│  └─────────────┘                                                       │
│         │                                                               │
│  ┌──────┴──────┐                                                       │
│  │ AgentActor  │──────────────────────────────────────────────────────┤
│  │ (API calls) │          (AgentEvent ──▶ StatePipe)                   │
│  └─────────────┘                                                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Pipe Architecture

### 1.1 Pipe Trait

```rust
/// Pipe trait — a unidirectional channel that transforms Input → Output.
/// Pipes are pure (no side effects) except where noted.
pub trait Pipe<Input> {
    type Output;
    fn pipe(&self, input: Input) -> Self::Output;
}
```

### 1.2 InputPipe

**Purpose**: Convert crossterm events to typed `Msg` variants.

```rust
/// InputPipe encapsulates the crossterm polling scope.
/// It runs in a spawned blocking task and sends Msg variants via channel.
pub struct InputPipe {
    msg_tx: mpsc::Sender<InputMsg>,
    cancel: CancellationToken,
}

/// Messages that InputPipe can emit
#[derive(Debug, Clone)]
pub enum InputMsg {
    /// Terminal keyboard event
    Key(KeyEvent),
    /// Terminal paste event
    Paste(String),
    /// Terminal resize event
    Resize(u16, u16),
}

impl InputPipe {
    /// Create a new InputPipe
    pub fn new(msg_tx: mpsc::Sender<InputMsg>, cancel: CancellationToken) -> Self {
        Self { msg_tx, cancel }
    }

    /// Run the input pipe. Consumes self.
    /// Polls crossterm events in a blocking task and sends them as InputMsg.
    pub async fn run(self) {
        let child_cancel = self.cancel.child_token();
        let msg_tx = self.msg_tx;

        let handle = tokio::task::spawn_blocking(move || {
            Self::poll_events(child_cancel, msg_tx);
        });

        if let Err(e) = handle.await {
            tracing::error!("[InputPipe] Error: {}", e);
        }
    }

    fn poll_events(cancel: CancellationToken, msg_tx: mpsc::Sender<InputMsg>) {
        use std::time::Duration;
        while !cancel.is_cancelled() {
            if crossterm::event::poll(Duration::from_millis(1)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    let msgs = match event {
                        crossterm::event::Event::Resize(w, h) => {
                            vec![InputMsg::Resize(w, h)]
                        }
                        crossterm::event::Event::Paste(text) => {
                            vec![InputMsg::Paste(text)]
                        }
                        crossterm::event::Event::Key(key) => {
                            vec![InputMsg::Key(key)]
                        }
                        _ => vec![],
                    };
                    for msg in msgs {
                        let _ = msg_tx.try_send(msg);
                    }
                }
            }
        }
    }
}
```

### 1.3 StatePipe

**Purpose**: Reducer pattern — transform `Msg` into state changes. No I/O here.

```rust
/// StatePipe is the application reducer.
/// It receives Msg variants and produces StateChange effects.
/// No side effects — pure state transformation.
pub struct StatePipe {
    state: AppState,
}

impl StatePipe {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Get current state (for ViewModelPipe)
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Process a message and return state changes + commands.
    /// This is the reducer — pure state transformation.
    pub fn process(&mut self, msg: Msg) -> StateChange {
        match msg {
            Msg::Submit => self.reduce_submit(),
            Msg::TextareaKey(key) => self.reduce_textarea_key(key),
            // ... other variants
            _ => StateChange::none(),
        }
    }

    /// Process an AgentEvent (from AgentActor)
    pub fn process_agent_event(&mut self, event: AgentEvent) -> StateChange {
        match event {
            AgentEvent::Message { role, content } => self.reduce_agent_message(&role, &content),
            // ... other variants
        }
    }

    /// Get mutable reference to state (for direct mutation by update functions)
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }
}

/// StateChange represents mutations to AppState + side effects to execute.
/// Multiple changes can be batched.
#[derive(Debug, Clone, Default)]
pub struct StateChange {
    pub state_mutations: Vec<StateMutation>,
    pub cmds: Vec<Cmd>,
    pub needs_render: bool,
}

impl StateChange {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_mutation(mut self, mutation: StateMutation) -> Self {
        self.state_mutations.push(mutation);
        self
    }

    pub fn with_cmd(mut self, cmd: Cmd) -> Self {
        self.cmds.push(cmd);
        self
    }

    pub fn with_render(mut self) -> Self {
        self.needs_render = true;
        self
    }

    pub fn merge(&mut self, other: StateChange) {
        self.state_mutations.extend(other.state_mutations);
        self.cmds.extend(other.cmds);
        self.needs_render = self.needs_render || other.needs_render;
    }
}

/// StateMutation is an enum of all possible state field mutations.
/// This replaces direct field access on AppState.
#[derive(Debug, Clone)]
pub enum StateMutation {
    // Chat mutations
    AppendMessage(MessageItem),
    UpdateLastAssistant(String),
    ClearMessages,
    SetTextareaText(String),
    TextareaInput(KeyEvent),
    TextareaInsertNewline,

    // Scroll mutations
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ResetScroll,

    // Agent mutations
    SetAgentRunning(bool),
    SetAgentStartTime(Option<Instant>),
    SetStatusHeader(Option<String>),
    SetStatusDetails(Option<String>),

    // Animation mutations
    TickAnimation,
    CursorBlink,

    // ... etc
}
```

### 1.4 ViewModelPipe

**Purpose**: Transform `AppState` into `ViewModels` using declarative builders.

```rust
/// ViewModelPipe transforms AppState into ViewModels using builders.
/// This is a pure transformation — no side effects.
pub struct ViewModelPipe {
    wrap_cache: WrapCache,
}

impl ViewModelPipe {
    pub fn new(wrap_cache: WrapCache) -> Self {
        Self { wrap_cache }
    }

    /// Build all view models from current state.
    pub fn build(&self, state: &AppState, palette: &CommandPalette) -> ViewModels {
        ViewModels {
            top_bar: TopBarBuilder::new()
                .with_state(&state.top_bar)
                .build(),
            message_list: FeedBuilder::new()
                .with_messages(&state.messages)
                .with_scroll(state.scroll.feed_offset)
                .with_animation(&state.animation)
                .with_wrap_cache(self.wrap_cache.clone())
                .build(),
            input_bar: InputBuilder::new()
                .with_textarea(&state.textarea)
                .with_prompt("\u{276F} ")
                .with_right_info(&state.input_right_info)
                .build(),
            status_bar: StatusBarBuilder::new()
                .with_mode(state.mode.clone())
                .with_model(state.current_model.clone())
                .with_token_usage(&state.session_token_usage)
                .with_status(&state.status_header, &state.status_details, state.status_start_time)
                .build(),
            agent_list: AgentListBuilder::new()
                .with_messages(&state.messages)
                .with_background_jobs(&state.background_jobs)
                .with_token_usage(state.session_token_usage.total_tokens as u64)
                .with_agent_running(state.agent_running)
                .with_braille_frame(state.animation.braille_frame)
                .build(),
            permission_modal: PermissionBuilder::new()
                .with_state(&state.permission_modal)
                .with_mode(&state.mode)
                .build(),
            onboarding: OnboardingBuilder::new()
                .with_state(state.onboarding.as_ref())
                .build(),
            command_palette: CommandPaletteBuilder::new()
                .with_palette(palette)
                .with_mode(&state.mode)
                .build(),
            session_tree: SessionTreeBuilder::new()
                .with_state(&state.session_tree)
                .with_mode(&state.mode)
                .build(),
            diff_viewer: DiffViewerBuilder::new()
                .with_diff(state.diff_viewer.as_ref())
                .build(),
            overlay: OverlayBuilder::new()
                .with_picker(state.model_picker.as_ref())
                .with_mode(&state.mode)
                .build(),
        }
    }
}
```

### 1.5 RenderPipe

**Purpose**: Transform `ViewModels` into terminal frames.

```rust
/// RenderPipe transforms ViewModels into terminal draw calls.
/// This is where terminal I/O happens (but abstracted for testability).
pub struct RenderPipe {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    config: TuiConfig,
}

impl RenderPipe {
    pub fn new(terminal: Terminal<CrosstermBackend<io::Stdout>>, config: TuiConfig) -> Self {
        Self { terminal, config }
    }

    /// Render a frame to the terminal.
    pub fn render(&mut self, vms: &ViewModels, area: Rect) -> io::Result<()> {
        let theme = &self.config.theme;
        let theme_colors = ThemeColors::from(theme);

        self.terminal.draw(|frame| {
            let buf = frame.buffer_mut();
            Self::render_frame(buf, vms, area, theme, &theme_colors);
        })
    }

    fn render_frame(
        buf: &mut Buffer,
        vms: &ViewModels,
        area: Rect,
        theme: &ThemeWrapper,
        theme_colors: &ThemeColors,
    ) {
        // Clear background
        Self::clear_background(buf, area, theme_colors.bg_base);

        // Calculate layout
        let input_height = Self::calculate_input_height(&vms.input_bar.textarea);
        let main_areas = Self::layout_main(area, input_height);

        // Render each component
        if self.config.show_top_bar {
            Component::render(&vms.top_bar, &vms.top_bar, main_areas[0], buf, theme);
        }
        Component::render(&vms.message_list, &vms.message_list, main_areas[1], buf, theme);
        Component::render(&vms.input_bar, &vms.input_bar, main_areas[2], buf, theme);

        if self.config.show_status_bar {
            Component::render(&vms.status_bar, &vms.status_bar, main_areas[3], buf, theme);
        }

        // Render overlays
        Self::render_overlays(buf, vms, area, theme, theme_colors);
    }
}
```

---

## 2. Actor Boundaries

Actors encapsulate **concurrent I/O operations**. They do NOT own UI state.

### 2.1 Actor Trait

```rust
/// Actor trait — implement this to create an actor that owns an I/O scope.
/// Actors are spawned as async tasks and communicate via channels.
/// They do NOT own UI state — they send messages TO the StatePipe.
pub trait Actor: Send + 'static {
    /// Messages this actor can receive
    type Msg: Send + Clone;

    /// Events this actor emits (sent to StatePipe)
    type Event: Send + Clone;

    /// Run the actor. Consumes self.
    fn run(self, msg_rx: mpsc::Receiver<Self::Msg>, event_tx: mpsc::Sender<Self::Event>);

    /// Human-readable name for logging
    fn name(&self) -> &'static str;
}
```

### 2.2 InputActor

```rust
/// InputActor encapsulates the crossterm polling scope.
/// It converts raw crossterm events into InputPipe messages.
/// NO UI state — sends messages TO InputPipe.
pub struct InputActor {
    cancel: CancellationToken,
}

impl InputActor {
    pub fn new(cancel: CancellationToken) -> Self {
        Self { cancel }
    }
}

impl Actor for InputActor {
    type Msg = (); // No incoming messages
    type Event = InputMsg; // Emits key/paste/resize events

    fn run(self, _msg_rx: mpsc::Receiver<()>, event_tx: mpsc::Sender<InputMsg>) {
        let child_cancel = self.cancel.child_token();
        let tx = event_tx;

        tokio::task::spawn_blocking(move || {
            while !child_cancel.is_cancelled() {
                if crossterm::event::poll(Duration::from_millis(1)).unwrap_or(false) {
                    if let Ok(event) = crossterm::event::read() {
                        let msg = match event {
                            crossterm::event::Event::Resize(w, h) => InputMsg::Resize(w, h),
                            crossterm::event::Event::Paste(text) => InputMsg::Paste(text),
                            crossterm::event::Event::Key(key) => InputMsg::Key(key),
                            _ => continue,
                        };
                        let _ = tx.try_send(msg);
                    }
                }
            }
        });
    }

    fn name(&self) -> &'static str {
        "InputActor"
    }
}
```

### 2.3 AgentActor

```rust
/// AgentActor encapsulates the LLM API call scope.
/// It receives agent messages and emits AgentEvent variants.
/// NO UI state — sends events TO StatePipe.
pub struct AgentActor {
    messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
}

impl AgentActor {
    pub fn new(/* ... */) -> Self { /* ... */ }
}

impl Actor for AgentActor {
    type Msg = AgentCommand; // Start, Stop, SendMessage
    type Event = AgentEvent; // Emits agent events

    fn run(self, msg_rx: mpsc::Receiver<AgentCommand>, event_tx: mpsc::Sender<AgentEvent>) {
        // Implementation in run_agent_loop
    }

    fn name(&self) -> &'static str {
        "AgentActor"
    }
}
```

### 2.4 TimerActor

```rust
/// TimerActor encapsulates the animation tick scope.
/// It emits Tick and CursorBlink events at fixed intervals.
/// NO UI state — sends messages TO StatePipe.
pub struct TimerActor {
    tick_interval: Duration,  // 80ms for animation
    cursor_interval: Duration, // 500ms for cursor blink
    cancel: CancellationToken,
}

impl TimerActor {
    pub fn new(tick_interval: Duration, cursor_interval: Duration, cancel: CancellationToken) -> Self {
        Self { tick_interval, cursor_interval, cancel }
    }
}

impl Actor for TimerActor {
    type Msg = (); // No incoming messages
    type Event = TimerMsg; // Tick or CursorBlink

    fn run(self, _msg_rx: mpsc::Receiver<()>, event_tx: mpsc::Sender<TimerMsg>) {
        tokio::spawn(async move {
            let mut tick_interval = tokio::time::interval(self.tick_interval);
            let mut cursor_interval = tokio::time::interval(self.cursor_interval);

            loop {
                tokio::select! {
                    _ = self.cancel.cancelled() => break,
                    _ = tick_interval.tick() => {
                        let _ = event_tx.send(TimerMsg::Tick).await;
                    }
                    _ = cursor_interval.tick() => {
                        let _ = event_tx.send(TimerMsg::CursorBlink).await;
                    }
                }
            }
        });
    }

    fn name(&self) -> &'static str {
        "TimerActor"
    }
}

#[derive(Debug, Clone)]
pub enum TimerMsg {
    Tick,
    CursorBlink,
}
```

---

## 3. Builder Trait

```rust
/// Builder trait — produce a ViewModel from state.
/// All builders follow this interface for consistency.
pub trait Builder<State, ViewModel> {
    fn new() -> Self
    where
        Self: Sized;

    fn with_state(&mut self, state: &State) -> &mut Self;

    fn build(&self) -> ViewModel;
}

/// Fluent builder for TopBar
pub struct TopBarBuilder {
    state: Option<TopBarState>,
}

impl TopBarBuilder {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn with_state(&mut self, state: &TopBarState) -> &mut Self {
        self.state = Some(state.clone());
        self
    }

    pub fn build(&self) -> TopBarViewModel {
        let state = self.state.as_ref().expect("state not set");
        TopBarViewModel {
            repo: state.repo.clone(),
            branch: state.branch.clone(),
            path: state.path.clone(),
            model: state.model.clone(),
            checks: TopBarChecks {
                passed: state.checks_passed,
                total: state.checks_total,
                percentage: state.percentage,
            },
            context: TopBarContext {
                badges: state.context_badges.clone(),
                window: state.context_window,
                estimated_tokens: state.estimated_tokens,
                percentage: state.context_pct,
            },
            agent_count: state.agent_count,
        }
    }
}

impl Default for TopBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluent builder for InputBar
pub struct InputBuilder {
    textarea: Option<ratatui_textarea::TextArea<'static>>,
    prompt: Option<String>,
    right_info: Option<String>,
}

impl InputBuilder {
    pub fn new() -> Self {
        Self {
            textarea: None,
            prompt: Some("\u{276F} ".to_string()),
            right_info: None,
        }
    }

    pub fn with_textarea(&mut self, textarea: &ratatui_textarea::TextArea<'static>) -> &mut Self {
        self.textarea = Some(textarea.clone());
        self
    }

    pub fn with_prompt(&mut self, prompt: &str) -> &mut Self {
        self.prompt = Some(prompt.to_string());
        self
    }

    pub fn with_right_info(&mut self, info: &str) -> &mut Self {
        self.right_info = Some(info.to_string());
        self
    }

    pub fn build(&self) -> InputBarViewModel {
        InputBarViewModel {
            textarea: self.textarea.clone().expect("textarea not set"),
            prompt: self.prompt.clone().expect("prompt not set"),
            right_info: self.right_info.clone().unwrap_or_default(),
        }
    }
}

/// Fluent builder for StatusBar
pub struct StatusBarBuilder {
    mode: Option<TuiMode>,
    current_model: Option<Option<String>>,
    session_token_usage: Option<TokenUsage>,
    status_header: Option<Option<String>>,
    status_details: Option<Option<String>>,
    status_start_time: Option<Option<Instant>>,
}

impl StatusBarBuilder {
    pub fn new() -> Self {
        Self {
            mode: None,
            current_model: None,
            session_token_usage: None,
            status_header: None,
            status_details: None,
            status_start_time: None,
        }
    }

    pub fn with_mode(&mut self, mode: TuiMode) -> &mut Self {
        self.mode = Some(mode);
        self
    }

    pub fn with_model(&mut self, model: Option<String>) -> &mut Self {
        self.current_model = Some(model);
        self
    }

    pub fn with_token_usage(&mut self, usage: &TokenUsage) -> &mut Self {
        self.session_token_usage = Some(usage.clone());
        self
    }

    pub fn with_status(
        &mut self,
        header: &Option<String>,
        details: &Option<String>,
        start_time: Option<Instant>,
    ) -> &mut Self {
        self.status_header = Some(header.clone());
        self.status_details = Some(details.clone());
        self.status_start_time = Some(start_time);
        self
    }

    pub fn build(&self) -> StatusBarViewModel {
        StatusBarViewModel {
            mode: self.mode.clone().expect("mode not set"),
            current_model: self.current_model.clone().expect("current_model not set"),
            session_token_usage: self.session_token_usage.clone().expect("session_token_usage not set"),
            status_header: self.status_header.clone().expect("status_header not set"),
            status_details: self.status_details.clone().expect("status_details not set"),
            status_start_time: self.status_start_time.expect("status_start_time not set"),
        }
    }
}

/// Fluent builder for MessageList/Feed
pub struct FeedBuilder {
    messages: Option<Vec<MessageItem>>,
    scroll_offset: Option<usize>,
    agent_running: Option<bool>,
    animation: Option<AnimationState>,
    wrap_cache: Option<WrapCache>,
}

impl FeedBuilder {
    pub fn new() -> Self {
        Self {
            messages: None,
            scroll_offset: None,
            agent_running: None,
            animation: None,
            wrap_cache: None,
        }
    }

    pub fn with_messages(&mut self, messages: &[MessageItem]) -> &mut Self {
        self.messages = Some(messages.to_vec());
        self
    }

    pub fn with_scroll(&mut self, offset: usize) -> &mut Self {
        self.scroll_offset = Some(offset);
        self
    }

    pub fn with_animation(&mut self, animation: &AnimationState) -> &mut Self {
        self.animation = Some(animation.clone());
        self
    }

    pub fn with_wrap_cache(&mut self, cache: WrapCache) -> &mut Self {
        self.wrap_cache = Some(cache);
        self
    }

    pub fn build(&self) -> MessageListViewModel {
        MessageListViewModel {
            messages: self.messages.clone().expect("messages not set"),
            scroll_offset: self.scroll_offset.unwrap_or(0),
            agent_running: self.agent_running.unwrap_or(false),
            animation: self.animation.clone().expect("animation not set"),
            wrap_cache: self.wrap_cache.clone().expect("wrap_cache not set"),
        }
    }
}

/// Fluent builder for PermissionModal
pub struct PermissionBuilder {
    tool: Option<String>,
    args: Option<String>,
    desc: Option<String>,
    tool_call_id: Option<String>,
    timeout_secs: Option<u64>,
    visible: bool,
}

impl PermissionBuilder {
    pub fn new() -> Self {
        Self {
            tool: None,
            args: None,
            desc: None,
            tool_call_id: None,
            timeout_secs: None,
            visible: false,
        }
    }

    pub fn with_state(&mut self, state: &PermissionModalState, mode: &TuiMode) -> &mut Self {
        if *mode == TuiMode::Permission && state.tool.is_some() {
            self.tool = state.tool.clone();
            self.args = state.args.clone();
            self.desc = state.desc.clone();
            self.tool_call_id = state.tool_call_id.clone();
            self.visible = true;

            // Calculate remaining timeout
            const TIMEOUT_SECS: u64 = 300;
            self.timeout_secs = state.timeout_start.map(|start| {
                TIMEOUT_SECS.saturating_sub(start.elapsed().as_secs())
            });
        }
        self
    }

    pub fn build(&self) -> Option<PermissionModalViewModel> {
        if !self.visible {
            return None;
        }

        Some(PermissionModalViewModel {
            tool: self.tool.clone().unwrap_or_default(),
            args: self.args.clone().unwrap_or_default(),
            desc: self.desc.clone().unwrap_or_default(),
            selected: 0,
            visible: true,
            timeout_secs: self.timeout_secs,
        })
    }
}

/// Fluent builder for Onboarding
pub struct OnboardingBuilder {
    state: Option<Onboarding>,
}

impl OnboardingBuilder {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn with_state(&mut self, state: Option<&Onboarding>) -> &mut Self {
        self.state = state.cloned();
        self
    }

    pub fn build(&self) -> Option<OnboardingViewModel> {
        let state = self.state.as_ref()?;
        Some(OnboardingViewModel {
            step: map_onboarding_step(&state.step),
            selected_item: state.selected_item,
            selected_provider: state.selected_provider,
            api_key_input: state.api_key_input.clone(),
            selected_model: state.selected_model,
            providers: state.providers.iter().map(|p| p.name.clone()).collect(),
            models: state.models.iter().map(|m| m.name.clone()).collect(),
            error_message: state.error_message.clone(),
        })
    }
}

/// Fluent builder for AgentList
pub struct AgentListBuilder {
    messages: Option<Vec<MessageItem>>,
    background_jobs: Option<Vec<BackgroundJob>>,
    token_usage: Option<u64>,
    agent_running: Option<bool>,
    braille_frame: Option<usize>,
}

impl AgentListBuilder {
    pub fn new() -> Self {
        Self {
            messages: None,
            background_jobs: None,
            token_usage: None,
            agent_running: None,
            braille_frame: None,
        }
    }

    pub fn with_messages(&mut self, messages: &[MessageItem]) -> &mut Self {
        self.messages = Some(messages.to_vec());
        self
    }

    pub fn with_background_jobs(&mut self, jobs: &[BackgroundJob]) -> &mut Self {
        self.background_jobs = Some(jobs.to_vec());
        self
    }

    pub fn with_token_usage(&mut self, tokens: u64) -> &mut Self {
        self.token_usage = Some(tokens);
        self
    }

    pub fn with_agent_running(&mut self, running: bool) -> &mut Self {
        self.agent_running = Some(running);
        self
    }

    pub fn with_braille_frame(&mut self, frame: usize) -> &mut Self {
        self.braille_frame = Some(frame);
        self
    }

    pub fn build(&self) -> AgentListViewModel {
        let messages = self.messages.as_ref().expect("messages not set");
        let plan_steps = extract_plan_steps(messages);

        let background_jobs = self.background_jobs.as_ref().expect("background_jobs not set");
        let running_jobs: Vec<_> = background_jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .cloned()
            .collect();

        let token_usage = self.token_usage.expect("token_usage not set");
        let session_usage = TokenUsage { /* from token_usage */ };

        AgentListViewModel {
            plan_steps,
            running_jobs,
            active_count: running_jobs.len(),
            tokens: token_usage,
            cost: session_usage.estimated_cost,
            agent_running: self.agent_running.expect("agent_running not set"),
            braille_frame: self.braille_frame.expect("braille_frame not set"),
        }
    }
}

/// Fluent builder for CommandPalette
pub struct CommandPaletteBuilder {
    palette: Option<CommandPalette>,
    mode: Option<TuiMode>,
}

impl CommandPaletteBuilder {
    pub fn new() -> Self {
        Self {
            palette: None,
            mode: None,
        }
    }

    pub fn with_palette(&mut self, palette: &CommandPalette) -> &mut Self {
        self.palette = Some(palette.clone());
        self
    }

    pub fn with_mode(&mut self, mode: &TuiMode) -> &mut Self {
        self.mode = Some(mode.clone());
        self
    }

    pub fn build(&self) -> Option<CommandPaletteViewModel> {
        let palette = self.palette.as_ref()?;
        let mode = self.mode.as_ref()?;

        if *mode != TuiMode::CommandPalette && !palette.open {
            return None;
        }

        Some(CommandPaletteViewModel {
            show: palette.open,
        })
    }
}

/// Fluent builder for Overlay
pub struct OverlayBuilder {
    picker: Option<ModelPicker>,
    mode: Option<TuiMode>,
}

impl OverlayBuilder {
    pub fn new() -> Self {
        Self {
            picker: None,
            mode: None,
        }
    }

    pub fn with_picker(&mut self, picker: Option<&ModelPicker>) -> &mut Self {
        self.picker = picker.cloned();
        self
    }

    pub fn with_mode(&mut self, mode: &TuiMode) -> &mut Self {
        self.mode = Some(mode.clone());
        self
    }

    pub fn build(&self) -> Option<OverlayViewModel> {
        let mode = self.mode.as_ref()?;
        if *mode != TuiMode::Overlay {
            return None;
        }

        Some(OverlayViewModel {
            title: String::new(),
            content: vec![],
            tabs: vec![],
            active_tab: 0,
            show_close: true,
        })
    }
}
```

---

## 4. AppState Decomposition

### 4.1 Current State: 33 Flat Fields

```rust
// CURRENT: 33 fields in a flat struct
pub struct AppState {
    pub messages: Vec<MessageItem>,
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub input_right_info: String,
    pub mode: TuiMode,
    pub running: bool,
    pub show_sidebar: bool,
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub top_bar: TopBarState,
    pub permission_modal: PermissionModalState,
    pub command_palette: CommandPaletteState,
    pub scroll: ScrollState,
    pub animation: AnimationState,
    pub diff_viewer: Option<DiffViewer>,
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub session_tree: SessionTreeNavigator,
    pub background_jobs: Vec<BackgroundJob>,
    pub onboarding: Option<Onboarding>,
    pub terminal_size: (u16, u16),
    pub clear_input_confirm: ClearInputConfirm,
    pub model_picker: Option<ModelPicker>,
    pub agent_start_time: Option<Instant>,
    pub input_history: Vec<String>,
    pub input_history_index: Option<usize>,
    pub input_draft: String,
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<Instant>,
    pub thinking_start: Option<Instant>,
    pub thinking_duration: Option<Duration>,
    pub is_thinking: bool,
    pub mock_mode: bool,
}
```

### 4.2 Proposed Decomposition: Sub-states by Domain

```rust
// ─── Sub-states by domain ───────────────────────────────────────────────────

/// Chat domain: message list, input, scroll, history
#[derive(Clone, Default)]
pub struct ChatState {
    pub messages: Vec<MessageItem>,
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub input_right_info: String,
    pub scroll: ScrollState,
    pub input_history: Vec<String>,
    pub input_history_index: Option<usize>,
    pub input_draft: String,
    pub clear_input_confirm: ClearInputConfirm,
}

/// Agent domain: agent lifecycle, permissions, status
#[derive(Clone, Default)]
pub struct AgentState {
    pub running: bool,
    pub current_model: Option<String>,
    pub agent_start_time: Option<Instant>,
    pub thinking_start: Option<Instant>,
    pub thinking_duration: Option<Duration>,
    pub is_thinking: bool,
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<Instant>,
    pub permission_modal: PermissionModalState,
    pub background_jobs: Vec<BackgroundJob>,
}

/// Layout domain: UI structure, overlays, modals
#[derive(Clone, Default)]
pub struct LayoutState {
    pub mode: TuiMode,
    pub show_sidebar: bool,
    pub terminal_size: (u16, u16),
    pub diff_viewer: Option<DiffViewer>,
    pub model_picker: Option<ModelPicker>,
    pub command_palette: CommandPaletteState,
    pub session_tree: SessionTreeNavigator,
    pub onboarding: Option<Onboarding>,
}

/// System domain: global flags, configuration
#[derive(Clone, Default)]
pub struct SystemState {
    pub running: bool,
    pub mock_mode: bool,
}

/// Animation domain: all animation/frame state
#[derive(Clone, Default)]
pub struct AnimationState {
    pub braille_frame: usize,
    pub rewind_braille_frame: usize,
    pub streaming_cursor_visible: bool,
    pub interrupt_fade_start: Option<Instant>,
}

/// Metrics domain: token usage tracking
#[derive(Clone, Default)]
pub struct MetricsState {
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
}

// ─── Composed AppState ──────────────────────────────────────────────────────

/// AppState is composed of domain-specific sub-states.
/// This makes it easier to reason about state mutations and test.
#[derive(Clone)]
pub struct AppState {
    pub chat: ChatState,
    pub agent: AgentState,
    pub layout: LayoutState,
    pub system: SystemState,
    pub animation: AnimationState,
    pub metrics: MetricsState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            chat: ChatState::default(),
            agent: AgentState::default(),
            layout: LayoutState::default(),
            system: SystemState::default(),
            animation: AnimationState::default(),
            metrics: MetricsState::default(),
        }
    }
}

impl AppState {
    /// Create AppState from legacy flat state (for migration)
    pub fn from_legacy(state: LegacyAppState) -> Self {
        Self {
            chat: ChatState {
                messages: state.messages,
                textarea: state.textarea,
                input_right_info: state.input_right_info,
                scroll: state.scroll,
                input_history: state.input_history,
                input_history_index: state.input_history_index,
                input_draft: state.input_draft,
                clear_input_confirm: state.clear_input_confirm,
            },
            agent: AgentState {
                running: state.agent_running,
                current_model: state.current_model,
                agent_start_time: state.agent_start_time,
                thinking_start: state.thinking_start,
                thinking_duration: state.thinking_duration,
                is_thinking: state.is_thinking,
                status_header: state.status_header,
                status_details: state.status_details,
                status_start_time: state.status_start_time,
                permission_modal: state.permission_modal,
                background_jobs: state.background_jobs,
            },
            layout: LayoutState {
                mode: state.mode,
                show_sidebar: state.show_sidebar,
                terminal_size: state.terminal_size,
                diff_viewer: state.diff_viewer,
                model_picker: state.model_picker,
                command_palette: state.command_palette,
                session_tree: state.session_tree,
                onboarding: state.onboarding,
            },
            system: SystemState {
                running: state.running,
                mock_mode: state.mock_mode,
            },
            animation: AnimationState {
                braille_frame: state.animation.braille_frame,
                rewind_braille_frame: state.animation.rewind_braille_frame,
                streaming_cursor_visible: state.animation.streaming_cursor_visible,
                interrupt_fade_start: state.animation.interrupt_fade_start,
            },
            metrics: MetricsState {
                token_usage: state.token_usage,
                session_token_usage: state.session_token_usage,
            },
        }
    }

    /// Convert back to legacy state (for backward compatibility during migration)
    pub fn to_legacy(&self) -> LegacyAppState {
        // ... conversion code
    }
}
```

### 4.3 Field Migration Map

| Current Field | Target Sub-state | Target Field |
|---------------|------------------|--------------|
| `messages` | `ChatState` | `messages` |
| `textarea` | `ChatState` | `textarea` |
| `input_right_info` | `ChatState` | `input_right_info` |
| `scroll` | `ChatState` | `scroll` |
| `input_history` | `ChatState` | `input_history` |
| `input_history_index` | `ChatState` | `input_history_index` |
| `input_draft` | `ChatState` | `input_draft` |
| `clear_input_confirm` | `ChatState` | `clear_input_confirm` |
| `agent_running` | `AgentState` | `running` |
| `current_model` | `AgentState` | `current_model` |
| `agent_start_time` | `AgentState` | `agent_start_time` |
| `thinking_start` | `AgentState` | `thinking_start` |
| `thinking_duration` | `AgentState` | `thinking_duration` |
| `is_thinking` | `AgentState` | `is_thinking` |
| `status_header` | `AgentState` | `status_header` |
| `status_details` | `AgentState` | `status_details` |
| `status_start_time` | `AgentState` | `status_start_time` |
| `permission_modal` | `AgentState` | `permission_modal` |
| `background_jobs` | `AgentState` | `background_jobs` |
| `mode` | `LayoutState` | `mode` |
| `show_sidebar` | `LayoutState` | `show_sidebar` |
| `terminal_size` | `LayoutState` | `terminal_size` |
| `diff_viewer` | `LayoutState` | `diff_viewer` |
| `model_picker` | `LayoutState` | `model_picker` |
| `command_palette` | `LayoutState` | `command_palette` |
| `session_tree` | `LayoutState` | `session_tree` |
| `onboarding` | `LayoutState` | `onboarding` |
| `running` | `SystemState` | `running` |
| `mock_mode` | `SystemState` | `mock_mode` |
| `animation` (fields) | `AnimationState` | (same fields) |
| `token_usage` | `MetricsState` | `token_usage` |
| `session_token_usage` | `MetricsState` | `session_token_usage` |

---

## 5. Message Types

### 5.1 Unified Message Enum

```rust
/// All messages that flow through the application.
/// Organized by domain for clarity.
#[derive(Debug, Clone)]
pub enum Msg {
    // ─── Chat Domain ───────────────────────────────────────────────
    Submit,
    TextareaKey(KeyEvent),
    InsertNewline,
    ClearInput,
    ClearInputConfirm,
    ClearChat,
    Paste(String),
    HistoryUp,
    HistoryDown,

    // ─── Navigation Domain ─────────────────────────────────────────
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ToggleSidebar,
    OpenCommandPalette,
    CloseModal,
    ConfirmModal,
    SwitchModel,

    // ─── Permission Domain ─────────────────────────────────────────
    PermissionConfirm,
    PermissionCancel,
    PermissionAlways,
    PermissionSkip,
    PermissionTimeout,

    // ─── Command Palette Domain ─────────────────────────────────────
    CommandPaletteFilter(char),
    CommandPaletteBackspace,
    CommandPaletteUp,
    CommandPaletteDown,
    CommandPaletteConfirm,
    CommandPaletteCancelArgument,
    DirectCommand(PaletteCommand),

    // ─── Session Tree Domain ────────────────────────────────────────
    ToggleSessionTree,
    SessionTreeUp,
    SessionTreeDown,
    SessionTreeConfirm,

    // ─── Onboarding Domain ──────────────────────────────────────────
    OnboardingNext,
    OnboardingBack,
    OnboardingNavigateUp,
    OnboardingNavigateDown,
    OnboardingSelectProvider(usize),
    OnboardingSelectModel(usize),
    OnboardingKeyInput(char),
    OnboardingKeyBackspace,
    OnboardingSearchInput(char),
    OnboardingSearchBackspace,
    OnboardingSubmit,
    OnboardingSkip,
    EnterOnboarding,

    // ─── Select/Overlay Domain ───────────────────────────────────────
    SelectUp,
    SelectDown,
    SelectConfirm,
    SelectToggleDetails,

    // ─── Agent Domain (external events) ─────────────────────────────
    AgentEvent(AgentEvent),

    // ─── Animation Domain ────────────────────────────────────────────
    Tick,
    CursorBlink,

    // ─── System Domain ──────────────────────────────────────────────
    Quit,
    Stop,
    Resize(u16, u16),

    // ─── State Initialization ───────────────────────────────────────
    SetGitInfo { repo: String, branch: String, path: String },
    SetTopBarMockChecks {
        checks_passed: Option<usize>,
        checks_total: Option<usize>,
        percentage: Option<f32>,
        context_badges: Vec<String>,
    },
    SetTopBarRealChecks { context_badges: Vec<String> },
    SetInputRightInfo(String),
    ModelsFetched(Vec<ModelInfo>),
    ModelsFetchFailed(String),
    CopyLastResponse,
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        // Manual equality implementation
    }
}
```

### 5.2 Commands (Side Effects)

```rust
/// Side effects returned by StatePipe to be executed by the runtime.
#[derive(Debug, Clone)]
pub enum Cmd {
    /// Spawn agent with message history
    SpawnAgent { messages: Vec<AgentMessage> },
    /// Send permission decision to agent
    SendPermission { decision: PermissionDecision },
    /// Execute slash command
    SlashCommand(SlashCommand),
    /// Save provider/model settings
    SaveSettings { provider: String, model: String, api_key: String },
    /// Fetch available models for provider
    FetchModels { provider_id: String, api_key: String },
    /// Rollback tool changes on permission cancel
    Rollback { tool_call_id: String },
    /// Interrupt running agent
    Interrupt,
}
```

---

## 6. Example: Keypress Flow

### 6.1 Flow: User Types "Hello" and presses Enter

```
Step 1: InputActor (crossterm) ──KeyEvent('h')──> InputPipe
Step 2: InputPipe ──InputMsg::Key('h')──> StatePipe
Step 3: StatePipe.process(InputMsg::Key('h'))
        ├─► StateMutation::TextareaInput(KeyEvent('h'))
        ├─► state.chat.textarea.input('h')
        └─► StateChange { needs_render: true }
Step 4: StatePipe ──StateChange──> ViewModelPipe
Step 5: ViewModelPipe.build(state)
        ├─► InputBuilder.with_textarea(&state.chat.textarea)
        │           .with_prompt("\u{276F} ")
        │           .with_right_info(&state.chat.input_right_info)
        │           .build()
        └─► InputBarViewModel { textarea: "h", prompt: "❯ ", ... }
Step 6: ViewModelPipe ──ViewModels──> RenderPipe
Step 7: RenderPipe.render(vms)
        └─► Terminal.draw(|frame| { ... })
```

```
Step 1: InputActor (crossterm) ──KeyEvent(Enter)──> InputPipe
Step 2: InputPipe ──InputMsg::Key(Enter)──> StatePipe
Step 3: StatePipe.process(Msg::TextareaKey(Enter))
        └─► BUT WAIT — Enter is converted to Submit via event_to_msg routing
Step 3: StatePipe.process(Msg::Submit)
        ├─► state.chat.textarea.lines() → "Hello"
        ├─► StateMutation::AppendMessage(MessageItem::User { text: "Hello", ... })
        ├─► state.chat.messages.push(...)
        ├─► StateMutation::SetAgentRunning(true)
        ├─► Cmd::SpawnAgent { messages: [...] }
        └─► StateChange { cmds: [SpawnAgent], needs_render: true }
Step 4: StatePipe ──StateChange + Cmd──> Runtime
Step 5: Runtime executes Cmd::SpawnAgent
        └─► AgentActor.run() starts
Step 6: StatePipe ──StateChange──> ViewModelPipe
Step 7: ViewModelPipe.build(state)
        ├─► FeedBuilder.with_messages(&state.chat.messages).build()
        │   └─► MessageListViewModel { messages: [User("Hello"), ...] }
        └─► AgentListBuilder.with_agent_running(true).build()
            └─► AgentListViewModel { agent_running: true, ... }
Step 8: ViewModelPipe ──ViewModels──> RenderPipe
Step 9: RenderPipe.render(vms) ──Frame──> Terminal
```

### 6.2 Flow: Agent Sends Streaming Message

```
Step 1: AgentActor (LLM API) ──AgentEvent::MessageUpdate──> StatePipe
Step 2: StatePipe.process_agent_event(MessageUpdate)
        ├─► StateMutation::UpdateLastAssistant("Hello, world")
        └─► StateChange { needs_render: true }
Step 3: TimerActor ──TimerMsg::Tick──> StatePipe
Step 4: StatePipe.process(TimerMsg::Tick)
        ├─► StateMutation::TickAnimation
        ├─► state.animation.braille_frame = (frame + 1) % 10
        └─► StateChange { needs_render: false } (batched)
Step 5: (on next animation tick)
Step 6: StatePipe ──StateChange──> ViewModelPipe
Step 7: ViewModelPipe.build(state)
        ├─► FeedBuilder.with_animation(&state.animation).build()
        │   └─► MessageListViewModel { animation.braille_frame: 5, ... }
        └─► InputBuilder.with_textarea(&state.chat.textarea).build()
Step 8: ViewModelPipe ──ViewModels──> RenderPipe
Step 9: RenderPipe.render(vms)
        └─► Terminal.draw(|frame| {
               // Animation frame 5 of braille pattern
               // Updated message text "Hello, world"
             })
```

---

## 7. Migration Plan

### Phase 1: Parallel Structure (Week 1)

**Goal**: Introduce new types without breaking existing code.

1. **Add new `AppState` struct** alongside existing one:
   ```rust
   // In state.rs
   mod new_state {
       pub struct AppState { /* decomposed */ }
   }
   ```

2. **Create `StatePipe` wrapper** that uses existing `update()`:
   ```rust
   pub struct StatePipe {
       state: AppState, // existing type
   }
   impl StatePipe {
       pub fn process(&mut self, msg: Msg) -> StateChange {
           let cmds = update(&mut self.state, &mut palette, msg);
           StateChange { cmds, needs_render: true }
       }
   }
   ```

3. **Add builders** for each ViewModel type:
   ```rust
   pub struct FeedBuilder { /* ... */ }
   impl Builder<Vec<MessageItem>, MessageListViewModel> for FeedBuilder { /* ... */ }
   ```

4. **Keep existing `ViewModels::from_render_state`** — make it use builders internally.

### Phase 2: State Decomposition (Week 2)

**Goal**: Replace flat AppState with decomposed sub-states.

1. **Define sub-state structs**:
   ```rust
   pub struct ChatState { /* ... */ }
   pub struct AgentState { /* ... */ }
   pub struct LayoutState { /* ... */ }
   pub struct SystemState { /* ... */ }
   pub struct AnimationState { /* ... */ }
   pub struct MetricsState { /* ... */ }
   ```

2. **Add `AppState::from_legacy()`** for backward compatibility.

3. **Migrate update functions** to operate on sub-states:
   ```rust
   // Before
   state.agent_running = true;

   // After
   state.agent.running = true;
   ```

4. **Update `RenderState::from()`** to pull from sub-states.

### Phase 3: Pipe Integration (Week 3)

**Goal**: Replace direct `tui.update()` calls with pipe-based flow.

1. **Create `InputPipe`** from existing `InputActor`:
   ```rust
   pub struct InputPipe { /* ... */ }
   ```

2. **Create `TimerActor`** (extract from `tui_run.rs` timers).

3. **Wire up pipes in main loop**:
   ```rust
   // Before
   while tui.state.running {
       let msg = msg_rx.recv().await;
       let cmds = tui.update(msg);
       tui.render();
   }

   // After
   let mut state_pipe = StatePipe::new(AppState::default());
   let view_model_pipe = ViewModelPipe::new(WrapCache::new());
   let mut render_pipe = RenderPipe::new(terminal, config);

   while state_pipe.state().system.running {
       tokio::select! {
           Some(input_msg) = input_rx.recv() => {
               let msg = convert_input_msg(input_msg);
               let change = state_pipe.process(msg);
               execute_cmds(change.cmds).await;
               if change.needs_render {
                   let vms = view_model_pipe.build(state_pipe.state(), &palette);
                   render_pipe.render(&vms)?;
               }
           }
           Some(timer_msg) = timer_rx.recv() => {
               let change = state_pipe.process_timer(timer_msg);
               // ...
           }
       }
   }
   ```

### Phase 4: Actor Refinement (Week 4)

**Goal**: Ensure actors own I/O scopes properly.

1. **Verify `InputActor`**:
   - Only polls crossterm
   - Sends `InputMsg` to channel
   - No UI state

2. **Verify `TimerActor`**:
   - Only manages timers
   - Sends `TimerMsg` to channel
   - No UI state

3. **Verify `AgentActor`**:
   - Only handles LLM API calls
   - Sends `AgentEvent` to channel
   - No UI state

### Phase 5: Cleanup (Week 5)

**Goal**: Remove dead code and old patterns.

1. **Delete dead actor framework** (`actors/framework.rs`):
   ```rust
   // Remove:
   // - Actor trait with JSON serialization
   // - ActorSystem with type erasure
   // - All actor implementations (MessageListActor, InputBarActor, etc.)
   ```

2. **Delete inline builder functions** in `view_models.rs`:
   ```rust
   // Remove:
   // - build_message_list_vm()
   // - build_input_bar_vm()
   // - build_status_bar_vm()
   // etc.
   ```

3. **Delete `RenderState`** — not needed with pipe architecture.

4. **Update `tui_run.rs`** to use new pipe system.

---

## 8. Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `crates/runie-tui/src/pipe.rs` | Pipe trait and implementations |
| `crates/runie-tui/src/actors/input.rs` | InputActor (already exists, refactor) |
| `crates/runie-tui/src/actors/timer.rs` | TimerActor for animation ticks |
| `crates/runie-tui/src/actors/agent.rs` | AgentActor for LLM scope |
| `crates/runie-tui/src/builder.rs` | Builder trait + all builders |
| `crates/runie-tui/src/state/new_state.rs` | Decomposed AppState |
| `crates/runie-tui/src/state/migration.rs` | Legacy to new conversion |

### Files to Modify

| File | Changes |
|------|---------|
| `crates/runie-tui/src/tui/state.rs` | Add sub-state structs, deprecate flat fields |
| `crates/runie-tui/src/tui/view_models.rs` | Make builders the source of truth |
| `crates/runie-tui/src/tui/update.rs` | Route through StatePipe |
| `crates/runie-tui/src/tui.rs` | Use RenderPipe |
| `crates/runie-cli/src/tui_run.rs` | Wire up pipes + actors |
| `crates/runie-cli/src/actors/framework.rs` | DELETE (dead code) |
| `crates/runie-cli/src/actors/mod.rs` | Remove dead exports |

### Files to Delete

| File | Reason |
|------|--------|
| `crates/runie-cli/src/actors/framework.rs` | Dead code — uses JSON serialization for type erasure |

---

## 9. Backward Compatibility

### RenderState Compatibility

During migration, `RenderState` can be generated from new `AppState`:

```rust
impl From<&AppState> for RenderState {
    fn from(state: &AppState) -> Self {
        Self {
            messages: state.chat.messages.clone(),
            textarea: state.chat.textarea.clone(),
            input_right_info: state.chat.input_right_info.clone(),
            mode: state.layout.mode.clone(),
            // ... etc
        }
    }
}
```

### Builder Compatibility

Existing `ViewModels::from_render_state()` becomes:

```rust
impl ViewModels {
    pub fn from_state(state: &AppState, palette: &CommandPalette, cache: WrapCache) -> Self {
        let pipe = ViewModelPipe::new(cache);
        pipe.build(state, palette)
    }
}
```

---

## 10. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod state_pipe_tests {
    use super::*;

    #[test]
    fn test_submit_adds_message() {
        let state = AppState::default();
        let mut pipe = StatePipe::new(state);

        pipe.process(Msg::TextareaKey(KeyEvent::Char('h')));
        pipe.process(Msg::TextareaKey(KeyEvent::Char('i')));
        pipe.process(Msg::Submit);

        let change = pipe.process(Msg::Submit);
        assert!(!change.cmds.is_empty());
        assert!(matches!(
            change.state_mutations.last(),
            Some(StateMutation::AppendMessage(MessageItem::User { .. }))
        ));
    }

    #[test]
    fn test_scroll_up_increments_offset() {
        let state = AppState::default();
        let mut pipe = StatePipe::new(state);

        let change = pipe.process(Msg::ScrollUp);
        assert!(matches!(
            change.state_mutations.first(),
            Some(StateMutation::ScrollUp)
        ));
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_top_bar_builder() {
        let state = TopBarState {
            repo: "my-repo".to_string(),
            branch: "main".to_string(),
            path: "src/lib.rs".to_string(),
            model: "gpt-4".to_string(),
            checks_passed: Some(3),
            checks_total: Some(5),
            percentage: Some(60.0),
            context_badges: vec!["Cargo.toml".to_string()],
            context_window: Some(128_000),
            estimated_tokens: Some(5000),
            context_pct: None,
            context_bar_pct: None,
            agent_count: None,
        };

        let vm = TopBarBuilder::new()
            .with_state(&state)
            .build();

        assert_eq!(vm.repo, "my-repo");
        assert_eq!(vm.checks.passed, Some(3));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_keypress_to_render_pipeline() {
    // Setup pipes
    let (input_tx, input_rx) = mpsc::channel(100);
    let (agent_tx, agent_rx) = mpsc::channel(100);
    let (timer_tx, timer_rx) = mpsc::channel(100);

    let state = AppState::default();
    let state_pipe = StatePipe::new(state);
    let view_model_pipe = ViewModelPipe::new(WrapCache::new());

    // Spawn actors
    let input_actor = InputActor::new(input_rx, CancellationToken::new());
    let timer_actor = TimerActor::new(
        Duration::from_millis(80),
        Duration::from_millis(500),
        CancellationToken::new(),
    );

    // Send keypress
    input_tx.send(InputMsg::Key(KeyEvent::Char('h'))).await.unwrap();

    // Verify state change
    // Verify view model built
    // Verify render called
}
```
