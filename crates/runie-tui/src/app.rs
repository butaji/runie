//! `App` — the top-level TUI controller.

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use runie_core::commands::{BuiltinCommandDisposition, MappableBuiltinCommand};
use runie_core::events::EventBus;
use runie_core::r#loop::LoopActor;
use runie_core::types::{AgentEvent, AgentMessage, Model};
use tokio::sync::{broadcast, mpsc, watch};

use crate::event_renderer::EventRenderer;
use crate::layout::chat_layout_with_prompt_height;
use crate::scrollback_actor::ScrollbackActor;
use crate::status_actor::StatusActor;
use crate::view::{
    chat_document_with_props, ChatViewProps, Element, HeaderViewProps, ViewDocument, ViewProps,
};
pub use crate::widgets::PaletteAction;
use crate::widgets::{
    FeedSnapshot, PromptOutcome, PromptSnapshot, PromptWidget, Scrollback, Status, StatusBar,
    StatusSnapshot, TuiSnapshot,
};
use runie_core::session::{SessionActor, SessionSnapshot, SessionStorageActor};
use runie_tui_model::project_event;
pub use runie_tui_model::{ui_messages_for_event, UiCommand, UiMsg, UiState};

#[derive(Debug)]
pub enum AppExit {
    Quit,
    Error(String),
}

#[path = "app_effort.rs"]
mod app_effort;
#[path = "app_projection.rs"]
mod app_projection;
use app_projection::*;

type UiMailbox = (UiMsg, tokio::sync::oneshot::Sender<()>);

#[derive(Clone)]
pub struct UiActor {
    tx: mpsc::Sender<UiMailbox>,
    snapshot: watch::Receiver<UiState>,
    commands: broadcast::Sender<UiCommand>,
    _owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
    /// Atomic counter incremented for every event drained from the bus
    /// subscriber. Tests rely on this to assert that queued `UiMsg`
    /// mailbox messages are serviced before a flood of broadcast events.
    #[allow(dead_code, reason = "read by tests via the actor's worker")]
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl UiActor {
    pub fn new(bus: &EventBus) -> Self {
        Self::new_with_welcome(bus, false)
    }

    pub fn new_with_welcome(bus: &EventBus, show_welcome: bool) -> Self {
        let initial = initial_ui_state(show_welcome);
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (commands, _) = broadcast::channel(32);
        let command_tx = commands.clone();
        let events = bus.subscribe();
        let event_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_for_worker = event_counter.clone();
        // The pause hook is `None` for production-owned actors so the
        // post-message branch in `run_ui_actor` compiles out; only the
        // direct regression test wires up a real `(Notify, Notify)` pair
        // to park the actor at the observation point.
        let (tx, owner) =
            runie_core::spawn_actor_worker!(32, |rx: mpsc::Receiver<UiMailbox>| async move {
                run_ui_actor(
                    rx,
                    events,
                    snapshot_tx,
                    command_tx,
                    initial,
                    counter_for_worker,
                    #[cfg(test)]
                    None,
                )
                .await;
            });
        Self {
            tx,
            snapshot,
            commands,
            _owner: owner,
            event_counter,
        }
    }

    pub async fn send(&self, message: UiMsg) {
        let _ = runie_core::mailbox_ack!(self.tx, |reply| (message, reply));
    }

    pub fn snapshot(&self) -> UiState {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe_commands(&self) -> broadcast::Receiver<UiCommand> {
        self.commands.subscribe()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "ui actor keeps the mailbox and event reductions explicit and adds a test-only pause hook"
)]
async fn run_ui_actor(
    mut rx: mpsc::Receiver<UiMailbox>,
    mut events: broadcast::Receiver<AgentEvent>,
    snapshot_tx: watch::Sender<UiState>,
    command_tx: broadcast::Sender<UiCommand>,
    initial: UiState,
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(test)] pause_hooks: Option<(
        std::sync::Arc<tokio::sync::Notify>,
        std::sync::Arc<tokio::sync::Notify>,
    )>,
) {
    use std::sync::atomic::Ordering;
    let mut state = initial;
    loop {
        tokio::select! {
            // Bias toward the mailbox: every interactive UI state
            // transition waits on a `UiMsg`, so the reducer must
            // service it before draining a flood of broadcast
            // events that have already arrived on `events`.
            biased;
            message = rx.recv() => {
                let Some((message, applied)) = message else { break };
                apply_ui_message(&mut state, &snapshot_tx, &command_tx, message, applied,
                    #[cfg(test)] pause_hooks.as_ref()).await;
            }
            event = events.recv() => {
                event_counter.fetch_add(1, Ordering::SeqCst);
                if let Ok(event) = event {
                    for message in project_event(&event).ui {
                        state = state.update(message);
                    }
                    let _ = snapshot_tx.send(state.clone());
                }
            }
        }
    }
}

async fn apply_ui_message(
    state: &mut UiState,
    snapshot_tx: &watch::Sender<UiState>,
    command_tx: &broadcast::Sender<UiCommand>,
    message: UiMsg,
    applied: tokio::sync::oneshot::Sender<()>,
    #[cfg(test)] pause_hooks: Option<&(
        std::sync::Arc<tokio::sync::Notify>,
        std::sync::Arc<tokio::sync::Notify>,
    )>,
) {
    let command = ui_command_for(state, &message);
    *state = state.clone().update(message);
    if let Some(command) = command {
        let _ = command_tx.send(command);
    }
    let _ = snapshot_tx.send(state.clone());
    let _ = applied.send(());
    #[cfg(test)]
    if let Some((message_done, actor_release)) = pause_hooks {
        message_done.notify_one();
        actor_release.notified().await;
    }
}

enum PromptMsg {
    Key(KeyEvent, tokio::sync::oneshot::Sender<PromptOutcome>),
    Clear(tokio::sync::oneshot::Sender<()>),
    CycleMode(tokio::sync::oneshot::Sender<()>),
    OpenFileSearch(tokio::sync::oneshot::Sender<()>),
    SetCaption(String, tokio::sync::oneshot::Sender<()>),
    SetPlaceholderVisible(bool, tokio::sync::oneshot::Sender<()>),
    SetTheme(
        runie_core::types::ThemeKind,
        tokio::sync::oneshot::Sender<()>,
    ),
    ApplyEvent(Box<AgentEvent>, tokio::sync::oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct PromptActor {
    tx: mpsc::Sender<PromptMsg>,
    snapshot: watch::Receiver<PromptSnapshot>,
    shared_snapshot: watch::Receiver<runie_core::SharedSnapshot<PromptSnapshot>>,
    _owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
    /// Atomic counter incremented for every event drained from the bus
    /// subscriber. Tests rely on this to assert that key mailbox messages
    /// are serviced before a flood of queued broadcast events.
    #[allow(dead_code, reason = "read by tests via the actor's worker")]
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl PromptActor {
    pub fn new(bus: &EventBus) -> Self {
        let initial = PromptWidget::new().model_snapshot();
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (shared_tx, shared_snapshot) = watch::channel(runie_core::SharedSnapshot::new(initial));
        let events = bus.subscribe();
        let event_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_for_worker = event_counter.clone();
        // The pause hook is `None` for production-owned actors so the
        // post-key branch in `run_prompt_actor` compiles out; only the
        // direct regression test wires up a real `(Notify, Notify)` pair
        // to park the actor at the observation point.
        let (tx, owner) =
            runie_core::spawn_actor_worker!(32, |rx: mpsc::Receiver<PromptMsg>| async move {
                run_prompt_actor(
                    rx,
                    events,
                    snapshot_tx,
                    shared_tx,
                    counter_for_worker,
                    #[cfg(test)]
                    None,
                )
                .await;
            });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _owner: owner,
            event_counter,
        }
    }

    async fn acknowledge<F>(&self, command: F)
    where
        F: FnOnce(tokio::sync::oneshot::Sender<()>) -> PromptMsg,
    {
        let _ = runie_core::mailbox_ack!(self.tx, command);
    }

    pub async fn handle_key(&self, key: KeyEvent) -> PromptOutcome {
        let (reply, result) = tokio::sync::oneshot::channel();
        if self.tx.send(PromptMsg::Key(key, reply)).await.is_err() {
            return PromptOutcome::Ignored;
        }
        result.await.unwrap_or(PromptOutcome::Ignored)
    }

    pub async fn clear(&self) {
        self.acknowledge(PromptMsg::Clear).await;
    }

    pub async fn cycle_mode(&self) {
        self.acknowledge(PromptMsg::CycleMode).await;
    }

    pub async fn set_placeholder_visible(&self, visible: bool) {
        self.acknowledge(|reply| PromptMsg::SetPlaceholderVisible(visible, reply))
            .await;
    }

    pub async fn set_theme(&self, theme: runie_core::types::ThemeKind) {
        self.acknowledge(|reply| PromptMsg::SetTheme(theme, reply))
            .await;
    }

    pub async fn apply_event(&self, event: AgentEvent) {
        self.acknowledge(|reply| PromptMsg::ApplyEvent(Box::new(event), reply))
            .await;
    }

    pub async fn open_file_search(&self) {
        self.acknowledge(PromptMsg::OpenFileSearch).await;
    }

    pub async fn set_model_caption(&self, caption: String) {
        self.acknowledge(|reply| PromptMsg::SetCaption(caption, reply))
            .await;
    }

    pub fn snapshot(&self) -> PromptWidget {
        PromptWidget::from_model_snapshot(self.snapshot.borrow().clone())
    }

    pub fn model_snapshot(&self) -> PromptSnapshot {
        self.snapshot.borrow().clone()
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "prompt actor keeps each event-to-state transition explicit and adds a test-only pause hook"
)]
async fn run_prompt_actor(
    mut rx: mpsc::Receiver<PromptMsg>,
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    snapshot_tx: watch::Sender<PromptSnapshot>,
    shared_tx: watch::Sender<runie_core::SharedSnapshot<PromptSnapshot>>,
    event_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(test)] pause_hooks: Option<(
        std::sync::Arc<tokio::sync::Notify>,
        std::sync::Arc<tokio::sync::Notify>,
    )>,
) {
    use std::sync::atomic::Ordering;
    let mut prompt = PromptWidget::new();
    loop {
        tokio::select! {
            biased;
            message = rx.recv() => {
                let Some(message) = message else { break };
                let is_key = matches!(message, PromptMsg::Key(..));
                handle_prompt_message(&mut prompt, message).await;
                prompt_shared::publish_prompt_snapshot(&snapshot_tx, &shared_tx, &prompt);
                if is_key {
                    #[cfg(test)]
                    if let Some((key_done, actor_release)) = pause_hooks.as_ref() {
                        key_done.notify_one();
                        actor_release.notified().await;
                    }
                }
            }
            event = events.recv() => {
                event_counter.fetch_add(1, Ordering::SeqCst);
                if apply_prompt_event(&mut prompt, event) {
                    prompt_shared::publish_prompt_snapshot(&snapshot_tx, &shared_tx, &prompt);
                }
            }
        }
    }
}

fn apply_prompt_event(
    prompt: &mut PromptWidget,
    event: Result<AgentEvent, tokio::sync::broadcast::error::RecvError>,
) -> bool {
    match event {
        Ok(AgentEvent::Reset) => {
            let snapshot = prompt.model_snapshot();
            *prompt = PromptWidget::new();
            prompt.set_theme(snapshot.theme);
            prompt.set_model_caption(snapshot.model_caption);
            true
        }
        Ok(AgentEvent::ThemeChanged { theme }) => {
            prompt.set_theme(theme);
            true
        }
        Ok(AgentEvent::ModelChanged { model }) if !model.name.is_empty() => {
            prompt.set_model_caption(format!("{} (high)", model.name));
            true
        }
        _ => false,
    }
}
#[path = "prompt_shared.rs"]
mod prompt_shared;

#[allow(
    clippy::too_many_lines,
    reason = "prompt mailbox keeps each event-to-state transition explicit"
)]
async fn handle_prompt_message(prompt: &mut PromptWidget, message: PromptMsg) {
    match message {
        PromptMsg::Key(key, reply) => {
            let _ = reply.send(prompt.handle_key(key));
        }
        PromptMsg::Clear(reply) => {
            prompt.clear();
            let _ = reply.send(());
        }
        PromptMsg::CycleMode(reply) => {
            prompt.cycle_mode();
            let _ = reply.send(());
        }
        PromptMsg::OpenFileSearch(reply) => {
            prompt.open_file_search_async().await;
            let _ = reply.send(());
        }
        PromptMsg::SetCaption(caption, reply) => {
            prompt.set_model_caption(caption);
            let _ = reply.send(());
        }
        PromptMsg::SetPlaceholderVisible(visible, reply) => {
            prompt.set_placeholder_visible(visible);
            let _ = reply.send(());
        }
        PromptMsg::SetTheme(theme, reply) => {
            prompt.set_theme(theme);
            let _ = reply.send(());
        }
        PromptMsg::ApplyEvent(event, reply) => {
            apply_prompt_message_event(prompt, *event);
            let _ = reply.send(());
        }
    }
}

fn apply_prompt_message_event(prompt: &mut PromptWidget, event: AgentEvent) {
    match event {
        AgentEvent::ThemeChanged { theme } => prompt.set_theme(theme),
        AgentEvent::ModelChanged { model } if !model.name.is_empty() => {
            prompt.set_model_caption(format!("{} (high)", model.name));
        }
        _ => {}
    }
}
pub struct App {
    pub prompt: PromptActor,
    pub status_actor: StatusActor,
    pub scrollback_actor: ScrollbackActor,
    pub session_actor: SessionActor,
    pub session_storage: SessionStorageActor,
    pub loop_actor: LoopActor,
    pub bus: EventBus,
    pub ui: UiActor,
    pub model_catalog: runie_core::model_catalog::ModelCatalogActor,
    pub provider_registry: runie_core::provider_registry::ProviderRegistryActor,
    pub command_actor: runie_core::command_actor::CommandActor,
    pub question_broker: runie_core::tools::UserQuestionBroker,
    pub approval_mode: runie_core::tools::ApprovalModeStore,
    pub background_actor: runie_core::background::BackgroundProcessActor,
    pub todo_actor: runie_core::tools::TodoActor,
    pub plugin_host: Option<runie_core::plugins::PluginHost>,
    submission_tx: SubmissionTx,
    _submission_owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
}
type Submission = (Vec<AgentMessage>, tokio::sync::oneshot::Sender<()>);
type SubmissionTx = mpsc::Sender<Submission>;

fn submission_actor(
    loop_actor: LoopActor,
) -> (
    SubmissionTx,
    std::sync::Arc<runie_core::task_owner::TaskOwner>,
) {
    runie_core::spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Submission>| async move {
        // The mailbox owns the task set: prompt runs must not occupy the
        // submission reducer, and they are cancelled together with the actor.
        let mut runs = tokio::task::JoinSet::new();
        while let Some((messages, accepted)) = rx.recv().await {
            while runs.try_join_next().is_some() {}
            let _ = accepted.send(());
            let loop_actor = loop_actor.clone();
            runs.spawn(async move {
                let _ = loop_actor
                    .prompt(messages, runie_core::types::AgentContext::default())
                    .await;
            });
        }
    })
}

#[path = "app_methods.rs"]
mod app_methods;
fn dialog_is_visible(ui: &UiState, id: &'static str) -> bool {
    let legacy_open = match id {
        "shortcuts" => ui.shortcuts_open,
        "commands" => ui.command_palette_open,
        "model" => ui.model_selector_open,
        "session" => ui.session_info_open,
        "changelog" => ui.changelog_open,
        "command-result" => ui.command_result.is_some(),
        _ => false,
    };
    legacy_open && (ui.dialog_stack.is_empty() || ui.dialog_stack.top_id() == Some(id))
}

fn compaction_token_estimates(snapshot: &SessionSnapshot) -> Vec<u64> {
    snapshot
        .entries
        .iter()
        .map(|entry| runie_core::session::estimate_message_tokens(&entry.message))
        .collect()
}

fn compaction_retained_tail(
    snapshot: &SessionSnapshot,
    preparation: &runie_core::session::CompactionPreparation,
) -> Vec<AgentMessage> {
    preparation
        .retained_indices
        .iter()
        .filter_map(|index| snapshot.entries.get(*index))
        .map(|entry| entry.message.clone())
        .collect()
}

#[path = "app_commands.rs"]
mod app_commands;
#[cfg(test)]
#[path = "app_palette_tests.rs"]
mod app_palette_tests;
#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
