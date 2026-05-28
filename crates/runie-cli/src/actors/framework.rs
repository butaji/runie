//! Actor framework for the runie CLI.
//!
//! # Design Principles
//!
//! - **Encapsulate state**: Each actor owns its state, not in a central AppState
//! - **Tell, don't ask**: Actors receive messages, mutate state, emit events
//! - **Single channel**: All actor communication via `mpsc::channel<ActorEvent>`
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    ActorSystem                           │
//! │  ┌──────────────────────────────────────────────────┐  │
//! │  │  Actors (state + handlers)                        │  │
//! │  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │  │
//! │  │  │ MessageList │ │  InputBar   │ │  StatusBar  │  │  │
//! │  │  │  Actor     │ │   Actor     │ │   Actor     │  │  │
//! │  │  └─────────────┘ └─────────────┘ └─────────────┘  │  │
//! │  │  ┌─────────────┐                                  │  │
//! │  │  │  TopBar     │                                  │  │
//! │  │  │   Actor     │                                  │  │
//! │  │  └─────────────┘                                  │  │
//! │  └──────────────────────────────────────────────────┘  │
//! │                         │                               │
//! │                    handle()                             │
//! │                         ▼                               │
//! │              Vec<<Actor::Event>>                        │
//! └─────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//!                     Event emission
//!                              │
//!                              ▼
//!                    Main event loop
//! ```

use std::collections::HashMap;
use tokio::sync::mpsc;
use serde::Serialize;

/// Actor trait — implement this to create an actor.
///
/// # Type Parameters
///
/// - `Msg`: Messages this actor can receive (must be Send + Clone)
/// - `Event`: Events this actor can emit (must be Send + Clone)
///
/// # Example
///
/// ```ignore
/// struct CounterActor {
///     count: usize,
/// }
///
/// impl Actor for CounterActor {
///     type Msg = CounterMsg;
///     type Event = CounterEvent;
///
///     fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event> {
///         match msg {
///             CounterMsg::Increment => {
///                 self.count += 1;
///                 vec![CounterEvent::Changed(self.count)]
///             }
///         }
///     }
///
///     fn name(&self) -> &'static str {
///         "Counter"
///     }
/// }
/// ```
pub trait Actor: Send + 'static {
    /// Messages this actor handles
    type Msg: Send + Clone;
    /// Events this actor emits
    type Event: Send + Clone;

    /// Handle a message and return events to emit.
    ///
    /// This is the only place state should be mutated.
    fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event>;

    /// Human-readable name for this actor (for logging/debugging)
    fn name(&self) -> &'static str;
}

/// Wrapper for any actor error
#[derive(Debug, Clone)]
pub struct ActorError {
    pub actor: &'static str,
    pub message: String,
}

impl std::fmt::Display for ActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.actor, self.message)
    }
}

impl std::error::Error for ActorError {}

/// Events emitted by actors to the ActorSystem
///
/// The system routes these to the main event loop for processing.
#[derive(Debug, Clone)]
pub enum ActorEvent {
    /// Event from a specific actor (type-erased for routing)
    ActorEvent {
        actor: &'static str,
        event: String, // JSON serialization for type-safe routing
    },
    /// Error from an actor
    Error(ActorError),
}

// ═══════════════════════════════════════════════════════════════════════════════
// MessageList Actor
// ═══════════════════════════════════════════════════════════════════════════════

/// Messages for the MessageList actor
#[derive(Debug, Clone)]
pub enum MessageListMsg {
    /// Add a new message
    AddMessage(MessageItem),
    /// Update an existing message by index
    UpdateMessage { index: usize, content: String },
    /// Append content to the last message (streaming)
    AppendToLastMessage(String),
    /// Remove a message by index
    RemoveMessage { index: usize },
    /// Clear all messages
    Clear,
    /// Set scroll offset
    SetScrollOffset(i32),
    /// Scroll to bottom (offset = 0)
    ScrollToBottom,
    /// Set pending tool calls
    SetPendingToolCalls(Vec<ToolCall>),
    /// Add a pending tool call
    AddPendingToolCall(ToolCall),
    /// Remove pending tool call
    RemovePendingToolCall { id: String },
}

/// Events emitted by the MessageList actor
#[derive(Debug, Clone, Serialize)]
pub enum MessageListEvent {
    /// Messages changed (list modified)
    MessagesChanged(Vec<MessageItem>),
    /// Scroll offset changed
    ScrollOffsetChanged(i32),
    /// Tool calls pending
    ToolCallsPending(Vec<ToolCall>),
    /// Error event
    Error(String),
}

/// A message item in the list
#[derive(Debug, Clone, Serialize)]
pub struct MessageItem {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
}

/// State for the MessageList actor
#[derive(Default)]
pub struct MessageListState {
    pub messages: Vec<MessageItem>,
    pub scroll_offset: i32,
    pub pending_tool_calls: Vec<ToolCall>,
}

impl MessageListState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// MessageList actor — manages the message feed and scroll state
pub struct MessageListActor {
    state: MessageListState,
}

impl MessageListActor {
    pub fn new() -> Self {
        Self {
            state: MessageListState::new(),
        }
    }

    pub fn with_messages(messages: Vec<MessageItem>) -> Self {
        Self {
            state: MessageListState {
                messages,
                ..Default::default()
            },
        }
    }

    /// Get current state snapshot (for rendering)
    pub fn state(&self) -> &MessageListState {
        &self.state
    }

    fn handle_add_message(&mut self, item: MessageItem) -> Vec<MessageListEvent> {
        self.state.messages.push(item);
        self.state.scroll_offset = 0; // Auto-scroll to bottom on new message
        vec![
            MessageListEvent::MessagesChanged(self.state.messages.clone()),
            MessageListEvent::ScrollOffsetChanged(self.state.scroll_offset),
        ]
    }

    fn handle_update_message(&mut self, index: usize, content: String) -> Vec<MessageListEvent> {
        if let Some(msg) = self.state.messages.get_mut(index) {
            msg.content = content;
            vec![MessageListEvent::MessagesChanged(self.state.messages.clone())]
        } else {
            vec![MessageListEvent::Error(format!(
                "Invalid message index: {}",
                index
            ))]
        }
    }

    fn handle_append_to_last(&mut self, content: String) -> Vec<MessageListEvent> {
        if let Some(msg) = self.state.messages.last_mut() {
            msg.content.push_str(&content);
            vec![MessageListEvent::MessagesChanged(self.state.messages.clone())]
        } else {
            vec![]
        }
    }

    fn handle_remove_message(&mut self, index: usize) -> Vec<MessageListEvent> {
        if index < self.state.messages.len() {
            self.state.messages.remove(index);
            vec![MessageListEvent::MessagesChanged(self.state.messages.clone())]
        } else {
            vec![MessageListEvent::Error(format!(
                "Invalid message index: {}",
                index
            ))]
        }
    }

    fn handle_clear(&mut self) -> Vec<MessageListEvent> {
        self.state.messages.clear();
        self.state.scroll_offset = 0;
        vec![MessageListEvent::MessagesChanged(Vec::new())]
    }

    fn handle_set_scroll_offset(&mut self, offset: i32) -> Vec<MessageListEvent> {
        self.state.scroll_offset = offset;
        vec![MessageListEvent::ScrollOffsetChanged(offset)]
    }

    fn handle_scroll_to_bottom(&mut self) -> Vec<MessageListEvent> {
        self.state.scroll_offset = 0;
        vec![MessageListEvent::ScrollOffsetChanged(0)]
    }

    fn handle_set_pending_tool_calls(&mut self, calls: Vec<ToolCall>) -> Vec<MessageListEvent> {
        self.state.pending_tool_calls = calls;
        vec![MessageListEvent::ToolCallsPending(self.state.pending_tool_calls.clone())]
    }

    fn handle_add_pending_tool_call(&mut self, call: ToolCall) -> Vec<MessageListEvent> {
        self.state.pending_tool_calls.push(call);
        vec![MessageListEvent::ToolCallsPending(self.state.pending_tool_calls.clone())]
    }

    fn handle_remove_pending_tool_call(&mut self, id: String) -> Vec<MessageListEvent> {
        self.state.pending_tool_calls.retain(|tc| tc.id != id);
        vec![MessageListEvent::ToolCallsPending(self.state.pending_tool_calls.clone())]
    }
}

impl Actor for MessageListActor {
    type Msg = MessageListMsg;
    type Event = MessageListEvent;

    fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event> {
        match msg {
            MessageListMsg::AddMessage(item) => self.handle_add_message(item),
            MessageListMsg::UpdateMessage { index, content } => {
                self.handle_update_message(index, content)
            }
            MessageListMsg::AppendToLastMessage(content) => {
                self.handle_append_to_last(content)
            }
            MessageListMsg::RemoveMessage { index } => self.handle_remove_message(index),
            MessageListMsg::Clear => self.handle_clear(),
            MessageListMsg::SetScrollOffset(offset) => self.handle_set_scroll_offset(offset),
            MessageListMsg::ScrollToBottom => self.handle_scroll_to_bottom(),
            MessageListMsg::SetPendingToolCalls(calls) => {
                self.handle_set_pending_tool_calls(calls)
            }
            MessageListMsg::AddPendingToolCall(call) => {
                self.handle_add_pending_tool_call(call)
            }
            MessageListMsg::RemovePendingToolCall { id } => {
                self.handle_remove_pending_tool_call(id)
            }
        }
    }

    fn name(&self) -> &'static str {
        "MessageList"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// InputBar Actor
// ═══════════════════════════════════════════════════════════════════════════════

/// Messages for the InputBar actor
#[derive(Debug, Clone)]
pub enum InputBarMsg {
    /// Set input text
    SetText(String),
    /// Append text to current input
    AppendText(String),
    /// Clear the input
    Clear,
    /// Set cursor position
    SetCursorPosition(usize),
    /// Move cursor left
    CursorLeft,
    /// Move cursor right
    CursorRight,
    /// Delete character before cursor
    DeleteBack,
    /// Delete character after cursor
    DeleteForward,
    /// Insert a character at cursor
    InsertChar(char),
    /// Insert newline
    InsertNewline,
    /// Set right info (model/provider display)
    SetRightInfo(String),
    /// Set whether agent is running (shows different cursor)
    SetAgentRunning(bool),
}

/// Events emitted by the InputBar actor
#[derive(Debug, Clone, Serialize)]
pub enum InputBarEvent {
    /// Input text changed
    TextChanged { text: String, cursor: usize },
    /// Cursor position changed
    CursorChanged(usize),
    /// Right info changed
    RightInfoChanged(String),
    /// Agent running state changed
    AgentRunningChanged(bool),
    /// Error event
    Error(String),
}

/// State for the InputBar actor
#[derive(Default)]
pub struct InputBarState {
    pub text: String,
    pub cursor: usize,
    pub right_info: String,
    pub agent_running: bool,
}

impl InputBarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cursor_to_bytes(&self) -> usize {
        self.text[..self.cursor.min(self.text.len())].chars().count()
    }
}

/// InputBar actor — manages the textarea/input state
pub struct InputBarActor {
    state: InputBarState,
}

impl InputBarActor {
    pub fn new() -> Self {
        Self {
            state: InputBarState::new(),
        }
    }

    /// Get current state snapshot (for rendering)
    pub fn state(&self) -> &InputBarState {
        &self.state
    }

    fn handle_set_text(&mut self, text: String) -> Vec<InputBarEvent> {
        self.state.text = text;
        self.state.cursor = self.state.text.len();
        vec![InputBarEvent::TextChanged {
            text: self.state.text.clone(),
            cursor: self.state.cursor,
        }]
    }

    fn handle_append_text(&mut self, text: String) -> Vec<InputBarEvent> {
        self.state.text.push_str(&text);
        vec![InputBarEvent::TextChanged {
            text: self.state.text.clone(),
            cursor: self.state.cursor,
        }]
    }

    fn handle_clear(&mut self) -> Vec<InputBarEvent> {
        self.state.text.clear();
        self.state.cursor = 0;
        vec![InputBarEvent::TextChanged {
            text: String::new(),
            cursor: 0,
        }]
    }

    fn handle_set_cursor_position(&mut self, pos: usize) -> Vec<InputBarEvent> {
        self.state.cursor = pos.min(self.state.text.len());
        vec![InputBarEvent::CursorChanged(self.state.cursor)]
    }

    fn handle_cursor_left(&mut self) -> Vec<InputBarEvent> {
        if self.state.cursor > 0 {
            self.state.cursor -= 1;
        }
        vec![InputBarEvent::CursorChanged(self.state.cursor)]
    }

    fn handle_cursor_right(&mut self) -> Vec<InputBarEvent> {
        if self.state.cursor < self.state.text.len() {
            self.state.cursor += 1;
        }
        vec![InputBarEvent::CursorChanged(self.state.cursor)]
    }

    fn handle_delete_back(&mut self) -> Vec<InputBarEvent> {
        if self.state.cursor > 0 {
            let new_cursor = self.state.cursor - 1;
            self.state.text.remove(new_cursor);
            self.state.cursor = new_cursor;
            vec![InputBarEvent::TextChanged {
                text: self.state.text.clone(),
                cursor: self.state.cursor,
            }]
        } else {
            vec![]
        }
    }

    fn handle_delete_forward(&mut self) -> Vec<InputBarEvent> {
        if self.state.cursor < self.state.text.len() {
            self.state.text.remove(self.state.cursor);
            vec![InputBarEvent::TextChanged {
                text: self.state.text.clone(),
                cursor: self.state.cursor,
            }]
        } else {
            vec![]
        }
    }

    fn handle_insert_char(&mut self, ch: char) -> Vec<InputBarEvent> {
        self.state.text.insert(self.state.cursor, ch);
        self.state.cursor += 1;
        vec![InputBarEvent::TextChanged {
            text: self.state.text.clone(),
            cursor: self.state.cursor,
        }]
    }

    fn handle_insert_newline(&mut self) -> Vec<InputBarEvent> {
        self.state.text.insert(self.state.cursor, '\n');
        self.state.cursor += 1;
        vec![InputBarEvent::TextChanged {
            text: self.state.text.clone(),
            cursor: self.state.cursor,
        }]
    }

    fn handle_set_right_info(&mut self, info: String) -> Vec<InputBarEvent> {
        self.state.right_info = info;
        vec![InputBarEvent::RightInfoChanged(self.state.right_info.clone())]
    }

    fn handle_set_agent_running(&mut self, running: bool) -> Vec<InputBarEvent> {
        self.state.agent_running = running;
        vec![InputBarEvent::AgentRunningChanged(running)]
    }
}

impl Actor for InputBarActor {
    type Msg = InputBarMsg;
    type Event = InputBarEvent;

    fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event> {
        match msg {
            InputBarMsg::SetText(text) => self.handle_set_text(text),
            InputBarMsg::AppendText(text) => self.handle_append_text(text),
            InputBarMsg::Clear => self.handle_clear(),
            InputBarMsg::SetCursorPosition(pos) => self.handle_set_cursor_position(pos),
            InputBarMsg::CursorLeft => self.handle_cursor_left(),
            InputBarMsg::CursorRight => self.handle_cursor_right(),
            InputBarMsg::DeleteBack => self.handle_delete_back(),
            InputBarMsg::DeleteForward => self.handle_delete_forward(),
            InputBarMsg::InsertChar(ch) => self.handle_insert_char(ch),
            InputBarMsg::InsertNewline => self.handle_insert_newline(),
            InputBarMsg::SetRightInfo(info) => self.handle_set_right_info(info),
            InputBarMsg::SetAgentRunning(running) => self.handle_set_agent_running(running),
        }
    }

    fn name(&self) -> &'static str {
        "InputBar"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// StatusBar Actor
// ═══════════════════════════════════════════════════════════════════════════════

/// Messages for the StatusBar actor
#[derive(Debug, Clone)]
pub enum StatusBarMsg {
    /// Set the current mode
    SetMode(StatusBarMode),
    /// Set the current model display
    SetCurrentModel(Option<String>),
    /// Set token usage
    SetTokenUsage(TokenUsage),
    /// Set session token usage
    SetSessionTokenUsage(TokenUsage),
    /// Add a background job
    AddBackgroundJob(BackgroundJob),
    /// Remove a background job
    RemoveBackgroundJob { id: String },
    /// Update animation frame
    Tick,
    /// Toggle cursor blink
    CursorBlink,
}

/// Events emitted by the StatusBar actor
#[derive(Debug, Clone, Serialize)]
pub enum StatusBarEvent {
    /// Status bar state changed
    StateChanged(StatusBarState),
    /// Animation frame changed
    AnimationFrame { braille_frame: usize, cursor_visible: bool },
    /// Error event
    Error(String),
}

/// Background job display info
#[derive(Debug, Clone, Serialize)]
pub struct BackgroundJob {
    pub id: String,
    pub name: String,
    pub progress: Option<f32>,
}

/// Token usage tracking
#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Mode for the status bar
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum StatusBarMode {
    Chat,
    Permission,
    Select,
    CommandPalette,
    DiffViewer,
    SessionTree,
    Onboarding,
}

impl Default for StatusBarMode {
    fn default() -> Self {
        StatusBarMode::Chat
    }
}

/// State for the StatusBar actor
#[derive(Debug, Clone, Default, Serialize)]
pub struct StatusBarState {
    pub mode: StatusBarMode,
    pub current_model: Option<String>,
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub background_jobs: Vec<BackgroundJob>,
    pub braille_frame: usize,
    pub cursor_visible: bool,
    pub streaming_cursor_visible: bool,
}

impl StatusBarState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// StatusBar actor — manages status display state
pub struct StatusBarActor {
    state: StatusBarState,
}

impl StatusBarActor {
    pub fn new() -> Self {
        Self {
            state: StatusBarState::new(),
        }
    }

    /// Get current state snapshot (for rendering)
    pub fn state(&self) -> &StatusBarState {
        &self.state
    }

    fn handle_set_mode(&mut self, mode: StatusBarMode) -> Vec<StatusBarEvent> {
        self.state.mode = mode;
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_set_current_model(&mut self, model: Option<String>) -> Vec<StatusBarEvent> {
        self.state.current_model = model;
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_set_token_usage(&mut self, usage: TokenUsage) -> Vec<StatusBarEvent> {
        self.state.token_usage = usage;
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_set_session_token_usage(&mut self, usage: TokenUsage) -> Vec<StatusBarEvent> {
        self.state.session_token_usage = usage;
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_add_background_job(&mut self, job: BackgroundJob) -> Vec<StatusBarEvent> {
        // Avoid duplicates
        if !self.state.background_jobs.iter().any(|j| j.id == job.id) {
            self.state.background_jobs.push(job);
        }
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_remove_background_job(&mut self, id: String) -> Vec<StatusBarEvent> {
        self.state.background_jobs.retain(|j| j.id != id);
        vec![StatusBarEvent::StateChanged(self.state.clone())]
    }

    fn handle_tick(&mut self) -> Vec<StatusBarEvent> {
        self.state.braille_frame = (self.state.braille_frame + 1) % 10;
        vec![StatusBarEvent::AnimationFrame {
            braille_frame: self.state.braille_frame,
            cursor_visible: self.state.streaming_cursor_visible,
        }]
    }

    fn handle_cursor_blink(&mut self) -> Vec<StatusBarEvent> {
        self.state.cursor_visible = !self.state.cursor_visible;
        vec![StatusBarEvent::AnimationFrame {
            braille_frame: self.state.braille_frame,
            cursor_visible: self.state.cursor_visible,
        }]
    }
}

impl Actor for StatusBarActor {
    type Msg = StatusBarMsg;
    type Event = StatusBarEvent;

    fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event> {
        match msg {
            StatusBarMsg::SetMode(mode) => self.handle_set_mode(mode),
            StatusBarMsg::SetCurrentModel(model) => self.handle_set_current_model(model),
            StatusBarMsg::SetTokenUsage(usage) => self.handle_set_token_usage(usage),
            StatusBarMsg::SetSessionTokenUsage(usage) => self.handle_set_session_token_usage(usage),
            StatusBarMsg::AddBackgroundJob(job) => self.handle_add_background_job(job),
            StatusBarMsg::RemoveBackgroundJob { id } => self.handle_remove_background_job(id),
            StatusBarMsg::Tick => self.handle_tick(),
            StatusBarMsg::CursorBlink => self.handle_cursor_blink(),
        }
    }

    fn name(&self) -> &'static str {
        "StatusBar"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TopBar Actor
// ═══════════════════════════════════════════════════════════════════════════════

/// Messages for the TopBar actor
#[derive(Debug, Clone)]
pub enum TopBarMsg {
    /// Set git info
    SetGitInfo { repo: String, branch: String, path: String },
    /// Set model name
    SetModel(String),
    /// Set checks progress
    SetChecks {
        passed: Option<usize>,
        total: Option<usize>,
        percentage: Option<f32>,
    },
    /// Set context badges
    SetContextBadges(Vec<String>),
    /// Set context window info
    SetContextInfo {
        context_window: Option<usize>,
        estimated_tokens: Option<usize>,
    },
    /// Set agent count
    SetAgentCount(Option<usize>),
    /// Update from real git checks
    SetRealChecks { context_badges: Vec<String> },
}

/// Events emitted by the TopBar actor
#[derive(Debug, Clone, Serialize)]
pub enum TopBarEvent {
    /// Top bar state changed
    StateChanged(TopBarStateSnapshot),
    /// Error event
    Error(String),
}

/// Snapshot of top bar state for rendering
#[derive(Debug, Clone, Serialize)]
pub struct TopBarStateSnapshot {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub context_badges: Vec<String>,
    pub context_window: Option<usize>,
    pub estimated_tokens: Option<usize>,
}

/// State for the TopBar actor
#[derive(Default)]
pub struct TopBarActorState {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub context_badges: Vec<String>,
    pub context_window: Option<usize>,
    pub estimated_tokens: Option<usize>,
}

impl TopBarActorState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// TopBar actor — manages the top bar info display
pub struct TopBarActor {
    state: TopBarActorState,
}

impl TopBarActor {
    pub fn new() -> Self {
        Self {
            state: TopBarActorState::new(),
        }
    }

    /// Get current state snapshot (for rendering)
    pub fn state(&self) -> &TopBarActorState {
        &self.state
    }

    fn to_snapshot(&self) -> TopBarStateSnapshot {
        TopBarStateSnapshot {
            repo: self.state.repo.clone(),
            branch: self.state.branch.clone(),
            path: self.state.path.clone(),
            model: self.state.model.clone(),
            checks_passed: self.state.checks_passed,
            checks_total: self.state.checks_total,
            percentage: self.state.percentage,
            context_badges: self.state.context_badges.clone(),
            context_window: self.state.context_window,
            estimated_tokens: self.state.estimated_tokens,
        }
    }

    fn handle_set_git_info(&mut self, repo: String, branch: String, path: String) -> Vec<TopBarEvent> {
        self.state.repo = repo;
        self.state.branch = branch;
        self.state.path = path;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_model(&mut self, model: String) -> Vec<TopBarEvent> {
        self.state.model = model;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_checks(&mut self, passed: Option<usize>, total: Option<usize>, percentage: Option<f32>) -> Vec<TopBarEvent> {
        self.state.checks_passed = passed;
        self.state.checks_total = total;
        self.state.percentage = percentage;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_context_badges(&mut self, badges: Vec<String>) -> Vec<TopBarEvent> {
        self.state.context_badges = badges;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_context_info(&mut self, context_window: Option<usize>, estimated_tokens: Option<usize>) -> Vec<TopBarEvent> {
        self.state.context_window = context_window;
        self.state.estimated_tokens = estimated_tokens;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_agent_count(&mut self, count: Option<usize>) -> Vec<TopBarEvent> {
        // Agent count is not stored in TopBarActorState, just emit the current state
        let _ = count;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }

    fn handle_set_real_checks(&mut self, badges: Vec<String>) -> Vec<TopBarEvent> {
        self.state.context_badges = badges;
        vec![TopBarEvent::StateChanged(self.to_snapshot())]
    }
}

impl Actor for TopBarActor {
    type Msg = TopBarMsg;
    type Event = TopBarEvent;

    fn handle(&mut self, msg: Self::Msg) -> Vec<Self::Event> {
        match msg {
            TopBarMsg::SetGitInfo { repo, branch, path } => {
                self.handle_set_git_info(repo, branch, path)
            }
            TopBarMsg::SetModel(model) => self.handle_set_model(model),
            TopBarMsg::SetChecks { passed, total, percentage } => {
                self.handle_set_checks(passed, total, percentage)
            }
            TopBarMsg::SetContextBadges(badges) => self.handle_set_context_badges(badges),
            TopBarMsg::SetContextInfo { context_window, estimated_tokens } => {
                self.handle_set_context_info(context_window, estimated_tokens)
            }
            TopBarMsg::SetAgentCount(count) => self.handle_set_agent_count(count),
            TopBarMsg::SetRealChecks { context_badges } => {
                self.handle_set_real_checks(context_badges)
            }
        }
    }

    fn name(&self) -> &'static str {
        "TopBar"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ActorSystem
// ═══════════════════════════════════════════════════════════════════════════════

/// Actor system — owns all actors and routes messages
///
/// # Example
///
/// ```ignore
/// let (event_tx, event_rx) = mpsc::channel(100);
/// let mut system = ActorSystem::new(event_tx);
/// system.register(MessageListActor::new());
/// system.register(InputBarActor::new());
///
/// // In main loop:
/// let events = system.send(message_list_id, MessageListMsg::AddMessage(item));
/// for event in events {
///     // Process events
/// }
/// ```
pub struct ActorSystem {
    actors: HashMap<&'static str, Box<dyn ActorHandle>>,
    event_tx: mpsc::Sender<ActorEvent>,
}

trait ActorHandle: Send {
    fn name(&self) -> &'static str;
    fn handle(&mut self, msg: Box<dyn std::any::Any + Send>) -> Vec<Box<dyn std::any::Any + Send>>;
    fn handle_boxed(&mut self, msg: Box<dyn std::any::Any + Send>) -> Vec<Box<dyn std::any::Any + Send>>;
}

struct ActorWrapper<A: Actor> {
    actor: A,
}

impl<A: Actor> ActorHandle for ActorWrapper<A> {
    fn name(&self) -> &'static str {
        self.actor.name()
    }

    fn handle(&mut self, msg: Box<dyn std::any::Any + Send>) -> Vec<Box<dyn std::any::Any + Send>> {
        let msg = msg.downcast::<A::Msg>().expect("Invalid message type");
        let events = self.actor.handle(*msg);
        events.into_iter().map(|e| Box::new(e) as Box<dyn std::any::Any + Send>).collect()
    }

    fn handle_boxed(&mut self, msg: Box<dyn std::any::Any + Send>) -> Vec<Box<dyn std::any::Any + Send>> {
        self.handle(msg)
    }
}

impl ActorSystem {
    /// Create a new ActorSystem with the given event sender
    pub fn new(event_tx: mpsc::Sender<ActorEvent>) -> Self {
        Self {
            actors: HashMap::new(),
            event_tx,
        }
    }

    /// Register an actor with the system
    pub fn register<A: Actor>(&mut self, actor: A) {
        let name = actor.name();
        self.actors.insert(name, Box::new(ActorWrapper { actor }));
        tracing::debug!("[ActorSystem] Registered actor: {}", name);
    }

    /// Get a reference to an actor by name (for testing/debugging)
    ///
    /// Note: This returns the actor handle, not the typed actor.
    #[allow(dead_code)]
    pub fn get_actor(&self, name: &'static str) -> Option<&dyn ActorHandle> {
        self.actors.get(name).map(|b| b.as_ref())
    }

    /// Send a message to a specific actor by name
    ///
    /// Returns the events emitted by the actor.
    /// Events are also emitted to the system's event channel.
    pub fn tell<A: Actor>(&mut self, actor_name: &'static str, msg: A::Msg) -> Vec<A::Event>
    where
        A::Event: 'static + Serialize + std::fmt::Debug,
    {
        let wrapper = self.actors.get_mut(actor_name);
        let wrapper = match wrapper {
            Some(w) => w,
            None => {
                tracing::warn!(
                    "[ActorSystem] No actor found with name: {}",
                    actor_name
                );
                return vec![];
            }
        };

        tracing::debug!("[ActorSystem] Routing {:?} to {}", std::any::type_name::<A::Msg>(), actor_name);

        // We need to downcast and call handle, but our trait doesn't support this
        // directly. We use a type-erased approach.
        let msg_any = Box::new(msg) as Box<dyn std::any::Any + Send>;
        let events = wrapper.handle_boxed(msg_any);

        // Convert back to typed events and emit to channel
        let typed_events: Vec<A::Event> = events
            .into_iter()
            .filter_map(|e| e.downcast::<A::Event>().ok().map(|b| *b))
            .collect();

        // Emit events to the system channel
        for event in &typed_events {
            // Serialize event to JSON for type-safe routing
            let event_json = serde_json::to_string(event)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string());
            let actor_event = ActorEvent::ActorEvent {
                actor: actor_name,
                event: event_json,
            };
            if self.event_tx.try_send(actor_event).is_err() {
                tracing::warn!(
                    "[ActorSystem] Failed to emit event from {} (channel full)",
                    actor_name
                );
            }
        }

        typed_events
    }

    /// Broadcast a message to all actors
    ///
    /// Returns all events emitted by any actor.
    #[allow(dead_code)]
    pub fn broadcast<A: Actor + Clone + 'static>(&mut self, msg: A::Msg) -> Vec<A::Event>
    where
        A::Msg: Clone,
        A::Event: Serialize + std::fmt::Debug,
    {
        let mut all_events = vec![];
        // Collect names first to avoid borrow issues
        let names: Vec<_> = self.actors.keys().copied().collect();
        for name in names {
            let events = self.tell::<A>(name, msg.clone());
            all_events.extend(events);
        }
        all_events
    }

    /// Get the list of registered actor names
    #[allow(dead_code)]
    pub fn actor_names(&self) -> Vec<&'static str> {
        self.actors.keys().copied().collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Convenience re-exports
// ═══════════════════════════════════════════════════════════════════════════════

/// Module containing all concrete actor implementations
pub mod actors {
    pub use super::{InputBarActor, MessageListActor, StatusBarActor, TopBarActor};
}

/// Module containing all message types
pub mod msgs {
    pub use super::{
        InputBarMsg, MessageListMsg, StatusBarMsg, TopBarMsg,
    };
}

/// Module containing all event types
pub mod events {
    pub use super::{
        InputBarEvent, MessageListEvent, StatusBarEvent, TopBarEvent,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_list_actor_add_message() {
        let mut actor = MessageListActor::new();
        let item = MessageItem {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_calls: vec![],
        };
        let events = actor.handle(MessageListMsg::AddMessage(item));
        assert!(!events.is_empty());
        assert!(matches!(
            events[0],
            MessageListEvent::MessagesChanged(_)
        ));
        assert_eq!(actor.state().messages.len(), 1);
    }

    #[test]
    fn test_message_list_actor_scroll() {
        let mut actor = MessageListActor::new();
        assert_eq!(actor.state().scroll_offset, 0);

        let events = actor.handle(MessageListMsg::SetScrollOffset(10));
        assert!(matches!(
            events[0],
            MessageListEvent::ScrollOffsetChanged(10)
        ));
        assert_eq!(actor.state().scroll_offset, 10);

        let events = actor.handle(MessageListMsg::ScrollToBottom);
        assert!(matches!(
            events[0],
            MessageListEvent::ScrollOffsetChanged(0)
        ));
        assert_eq!(actor.state().scroll_offset, 0);
    }

    #[test]
    fn test_input_bar_actor_text() {
        let mut actor = InputBarActor::new();

        let events = actor.handle(InputBarMsg::SetText("Hello".to_string()));
        assert!(!events.is_empty());
        assert_eq!(actor.state().text, "Hello");
        assert_eq!(actor.state().cursor, 5);

        let events = actor.handle(InputBarMsg::InsertChar('!'));
        assert!(!events.is_empty());
        assert_eq!(actor.state().text, "Hello!");
        assert_eq!(actor.state().cursor, 6);

        let events = actor.handle(InputBarMsg::Clear);
        assert!(!events.is_empty());
        assert_eq!(actor.state().text, "");
        assert_eq!(actor.state().cursor, 0);
    }

    #[test]
    fn test_input_bar_actor_cursor() {
        let mut actor = InputBarActor::new();
        actor.handle(InputBarMsg::SetText("Hello".to_string())).clear();

        assert_eq!(actor.state().cursor, 5);

        actor.handle(InputBarMsg::CursorLeft);
        assert_eq!(actor.state().cursor, 4);

        actor.handle(InputBarMsg::CursorRight);
        assert_eq!(actor.state().cursor, 5);

        actor.handle(InputBarMsg::CursorRight); // Already at end
        assert_eq!(actor.state().cursor, 5);
    }

    #[test]
    fn test_status_bar_actor_mode() {
        let mut actor = StatusBarActor::new();
        assert_eq!(actor.state().mode, StatusBarMode::Chat);

        let events = actor.handle(StatusBarMsg::SetMode(StatusBarMode::Permission));
        assert!(!events.is_empty());
        assert_eq!(actor.state().mode, StatusBarMode::Permission);

        let events = actor.handle(StatusBarMsg::Tick);
        assert!(matches!(
            events[0],
            StatusBarEvent::AnimationFrame { .. }
        ));
        assert_eq!(actor.state().braille_frame, 1);
    }

    #[test]
    fn test_top_bar_actor_git_info() {
        let mut actor = TopBarActor::new();

        let events = actor.handle(TopBarMsg::SetGitInfo {
            repo: "my-repo".to_string(),
            branch: "main".to_string(),
            path: "src/lib.rs".to_string(),
        });

        assert!(!events.is_empty());
        assert_eq!(actor.state().repo, "my-repo");
        assert_eq!(actor.state().branch, "main");
        assert_eq!(actor.state().path, "src/lib.rs");
    }

    #[test]
    fn test_actor_system_registration() {
        let (tx, _rx) = mpsc::channel(100);
        let mut system = ActorSystem::new(tx);

        system.register(MessageListActor::new());
        system.register(InputBarActor::new());

        let names = system.actor_names();
        assert!(names.contains(&"MessageList"));
        assert!(names.contains(&"InputBar"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_actor_system_tell() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut system = ActorSystem::new(tx);

        system.register(MessageListActor::new());

        let item = MessageItem {
            role: MessageRole::User,
            content: "Test".to_string(),
            tool_calls: vec![],
        };

        let events = system.tell::<MessageListActor>("MessageList", MessageListMsg::AddMessage(item));

        // Check returned events
        assert!(!events.is_empty());

        // Check event was emitted to channel
        let actor_event = rx.try_recv().unwrap();
        assert!(matches!(actor_event, ActorEvent::ActorEvent { actor, .. } if actor == "MessageList"));
    }

    #[test]
    fn test_actor_system_unknown_actor() {
        let (tx, _rx) = mpsc::channel(100);
        let mut system = ActorSystem::new(tx);

        let events = system.tell::<MessageListActor>("NonExistent", MessageListMsg::Clear);
        assert!(events.is_empty());
    }

    #[test]
    fn test_tool_calls() {
        let mut actor = MessageListActor::new();

        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "read_file".to_string(),
            arguments: "{\"path\": \"test.rs\"}".to_string(),
            result: None,
        };

        let events = actor.handle(MessageListMsg::AddPendingToolCall(tool_call.clone()));
        assert!(!events.is_empty());
        assert!(matches!(
            events[0],
            MessageListEvent::ToolCallsPending(_)
        ));

        assert_eq!(actor.state().pending_tool_calls.len(), 1);
        assert_eq!(actor.state().pending_tool_calls[0].id, "call_123");

        let events = actor.handle(MessageListMsg::RemovePendingToolCall {
            id: "call_123".to_string(),
        });
        assert_eq!(actor.state().pending_tool_calls.len(), 0);
    }

    #[test]
    fn test_token_usage() {
        let mut actor = StatusBarActor::new();

        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        let events = actor.handle(StatusBarMsg::SetTokenUsage(usage.clone()));
        assert!(!events.is_empty());
        assert_eq!(actor.state().token_usage.total_tokens, 150);

        let session_usage = TokenUsage {
            prompt_tokens: 500,
            completion_tokens: 200,
            total_tokens: 700,
        };

        let events = actor.handle(StatusBarMsg::SetSessionTokenUsage(session_usage));
        assert_eq!(actor.state().session_token_usage.total_tokens, 700);
    }

    #[test]
    fn test_background_jobs() {
        let mut actor = StatusBarActor::new();

        let job = BackgroundJob {
            id: "job_1".to_string(),
            name: "Building...".to_string(),
            progress: Some(0.5),
        };

        let events = actor.handle(StatusBarMsg::AddBackgroundJob(job.clone()));
        assert!(!events.is_empty());
        assert_eq!(actor.state().background_jobs.len(), 1);

        // Adding duplicate should be ignored
        let events = actor.handle(StatusBarMsg::AddBackgroundJob(job));
        assert_eq!(actor.state().background_jobs.len(), 1);

        let events = actor.handle(StatusBarMsg::RemoveBackgroundJob {
            id: "job_1".to_string(),
        });
        assert_eq!(actor.state().background_jobs.len(), 0);
    }
}
