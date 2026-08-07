//! `App` — the top-level TUI controller.

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use runie_core::events::EventBus;
use runie_core::r#loop::LoopActor;
use runie_core::types::{AgentEvent, AgentMessage};
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
use runie_core::session::{SessionActor, SessionSnapshot};
pub use runie_tui_model::{ui_messages_for_event, UiCommand, UiMsg, UiState};

#[derive(Debug)]
pub enum AppExit {
    Quit,
    Error(String),
}

fn palette_command_for(state: &UiState, message: UiMsg) -> Option<UiCommand> {
    if !matches!(message, UiMsg::ActivateCommandPalette) {
        return None;
    }
    PaletteAction::selected_label(&state.command_palette_query, state.command_palette_index)
        .and_then(|entry| palette_action_for(entry).map(UiCommand::ActivatePaletteEntry))
}

fn palette_action_for(entry: &str) -> Option<PaletteAction> {
    PaletteAction::from_label(entry)
}

fn initial_ui_state(show_welcome: bool) -> UiState {
    if show_welcome {
        UiState::with_welcome()
    } else {
        UiState::new()
    }
}

#[derive(Clone)]
pub struct UiActor {
    tx: mpsc::Sender<(UiMsg, tokio::sync::oneshot::Sender<()>)>,
    snapshot: watch::Receiver<UiState>,
    commands: broadcast::Sender<UiCommand>,
    _owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
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
        let mut events = bus.subscribe();
        let (tx, owner) = runie_core::spawn_actor_worker!(32, |mut rx: mpsc::Receiver<(
            UiMsg,
            tokio::sync::oneshot::Sender<()>
        )>| async move {
            let mut state = initial;
            loop {
                tokio::select! {
                    message = rx.recv() => {
                        let Some((message, applied)) = message else { break };
                        let command = palette_command_for(&state, message);
                        state = state.update(message);
                        if let Some(command) = command {
                            let _ = command_tx.send(command);
                        }
                        let _ = snapshot_tx.send(state.clone());
                        let _ = applied.send(());
                    }
                    event = events.recv() => {
                        if let Ok(event) = event {
                            for message in ui_messages_for_event(&event) {
                                state = state.update(message);
                            }
                            let _ = snapshot_tx.send(state.clone());
                        }
                    }
                }
            }
        });
        Self {
            tx,
            snapshot,
            commands,
            _owner: owner,
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
    _owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
}

impl PromptActor {
    pub fn new(bus: &EventBus) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(PromptWidget::new().model_snapshot());
        let events = bus.subscribe();
        let (tx, owner) =
            runie_core::spawn_actor_worker!(32, |rx: mpsc::Receiver<PromptMsg>| async move {
                run_prompt_actor(rx, events, snapshot_tx).await;
            });
        Self {
            tx,
            snapshot,
            _owner: owner,
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

async fn run_prompt_actor(
    mut rx: mpsc::Receiver<PromptMsg>,
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    snapshot_tx: watch::Sender<PromptSnapshot>,
) {
    let mut prompt = PromptWidget::new();
    loop {
        tokio::select! {
            message = rx.recv() => {
                let Some(message) = message else { break };
                handle_prompt_message(&mut prompt, message).await;
                let _ = snapshot_tx.send(prompt.model_snapshot());
            }
            event = events.recv() => {
                match event {
                    Ok(AgentEvent::Reset) => {
                        let snapshot = prompt.model_snapshot();
                        prompt = PromptWidget::new();
                        prompt.set_theme(snapshot.theme);
                        prompt.set_model_caption(snapshot.model_caption);
                        let _ = snapshot_tx.send(prompt.model_snapshot());
                    }
                    Ok(AgentEvent::ThemeChanged { theme }) => {
                        prompt.set_theme(theme);
                        let _ = snapshot_tx.send(prompt.model_snapshot());
                    }
                    Ok(AgentEvent::ModelChanged { model }) if !model.name.is_empty() => {
                        prompt.set_model_caption(format!("{} (high)", model.name));
                        let _ = snapshot_tx.send(prompt.model_snapshot());
                    }
                    Ok(AgentEvent::ModelChanged { .. }) => {}
                    _ => {}
                }
            }
        }
    }
}

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
            match *event {
                AgentEvent::ThemeChanged { theme } => prompt.set_theme(theme),
                AgentEvent::ModelChanged { model } if !model.name.is_empty() => {
                    prompt.set_model_caption(format!("{} (high)", model.name));
                }
                _ => {}
            }
            let _ = reply.send(());
        }
    }
}

pub struct App {
    pub prompt: PromptActor,
    pub status_actor: StatusActor,
    pub scrollback_actor: ScrollbackActor,
    pub session_actor: SessionActor,
    pub loop_actor: LoopActor,
    pub bus: EventBus,
    pub ui: UiActor,
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

impl App {
    pub fn new(loop_actor: LoopActor, bus: EventBus) -> Self {
        let ui = UiActor::new(&bus);
        let (submission_tx, submission_owner) = submission_actor(loop_actor.clone());
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new(),
            // EventRenderer is the single live bus-delivery boundary. The
            // actor still owns the feed state; it receives acknowledged
            // reducer messages from the renderer, so no second subscription
            // can reduce the same core event concurrently.
            scrollback_actor: ScrollbackActor::new(),
            session_actor: SessionActor::new_with_bus(&bus),
            loop_actor,
            bus,
            ui,
            submission_tx,
            _submission_owner: submission_owner,
        }
    }

    pub fn new_with_welcome(loop_actor: LoopActor, bus: EventBus) -> Self {
        let ui = UiActor::new_with_welcome(&bus, true);
        let (submission_tx, submission_owner) = submission_actor(loop_actor.clone());
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new(),
            // Keep one event-to-feed path in the interactive app: the
            // renderer delivers core events to this actor's mailbox.
            scrollback_actor: ScrollbackActor::new(),
            session_actor: SessionActor::new_with_bus(&bus),
            loop_actor,
            bus,
            ui,
            submission_tx,
            _submission_owner: submission_owner,
        }
    }

    pub async fn toggle_shortcuts(&self) {
        self.ui.send(UiMsg::ToggleShortcuts).await;
    }

    /// Reset the core and every event-driven TUI projection through the loop
    /// actor's single reset boundary.
    pub async fn reset_session(&self) -> Result<(), runie_core::r#loop::LoopError> {
        self.loop_actor.reset().await
    }

    /// Deliver one typed theme event to every owning projection and await the
    /// three mailbox acknowledgements before returning. The coordinator is
    /// the single delivery boundary for this application command; each actor
    /// still owns and reduces only its own state.
    pub async fn set_theme(&self, theme: runie_core::types::ThemeKind) {
        let event = AgentEvent::ThemeChanged { theme };
        tokio::join!(
            self.prompt.apply_event(event.clone()),
            self.status_actor.apply_event(&event),
            self.scrollback_actor.apply_event(&event),
        );
    }

    pub async fn toggle_command_palette(&self) {
        self.ui.send(UiMsg::ToggleCommandPalette).await;
    }

    pub async fn command_palette_key(&self, msg: UiMsg) {
        self.ui.send(msg).await;
    }

    pub async fn activate_command_palette(&self) -> Option<String> {
        self.ui.send(UiMsg::ActivateCommandPalette).await;
        self.ui.snapshot().last_palette_command
    }

    pub fn subscribe_ui_commands(&self) -> broadcast::Receiver<UiCommand> {
        self.ui.subscribe_commands()
    }

    pub async fn hide_welcome(&self) {
        self.ui.send(UiMsg::HideWelcome).await;
    }

    pub async fn toggle_activity_fold(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ToggleActivityExpanded)
            .await;
    }

    /// Grok's `e` fold intent targets the active scrollback entry. Until the
    /// full cursor/navigation model lands, the actor's last tool block is the
    /// deterministic selected-entry fallback; an empty feed keeps the legacy
    /// activity-group fold behavior.
    pub async fn toggle_selected_tool_fold(&self) {
        let snapshot = self.scrollback_actor.snapshot();
        let tool_call_id = snapshot.selected_tool_id().map(str::to_owned).or_else(|| {
            snapshot
                .tool_blocks()
                .last()
                .map(|block| block.tool_call_id.clone())
        });
        if let Some(tool_call_id) = tool_call_id {
            self.scrollback_actor
                .apply(crate::widgets::ScrollbackMsg::ToggleToolMode(tool_call_id))
                .await;
        } else {
            self.toggle_activity_fold().await;
        }
    }

    pub async fn select_next_tool(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectNextTool)
            .await;
    }

    pub async fn select_previous_tool(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectPreviousTool)
            .await;
    }

    pub async fn select_next_entry(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectNextEntry)
            .await;
    }

    pub async fn select_previous_entry(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectPreviousEntry)
            .await;
    }

    pub async fn extend_selection(&self, delta: i32) {
        let snapshot = self.scrollback_actor.model_snapshot();
        let Some(current) = snapshot.selected_entry else {
            return;
        };
        let next = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current
                .saturating_add(delta as usize)
                .min(snapshot.lines.len().saturating_sub(1))
        };
        let anchor = snapshot.selection_anchor.unwrap_or(current);
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectRange { anchor, head: next })
            .await;
    }

    pub async fn scroll_scrollback_by(&self, lines: i32) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ScrollBy(lines))
            .await;
    }

    pub async fn mouse_selection_start(&self, position: crate::widgets::CellPosition) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionStart(position))
            .await;
    }

    pub async fn mouse_selection_extend(&self, position: crate::widgets::CellPosition) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionExtend(
                position,
            ))
            .await;
    }

    pub async fn mouse_selection_commit(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionCommit)
            .await;
    }

    pub async fn request_copy_selection(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::RequestCopySelection)
            .await;
    }

    pub async fn clear_copy_request(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ClearCopyRequest)
            .await;
    }

    /// Apply a feed update through the actor that owns the rendered snapshot.
    /// The mutex is a compatibility fallback for apps whose renderer is not
    /// running yet.
    pub async fn apply_scrollback(&self, message: crate::widgets::ScrollbackMsg) {
        self.scrollback_actor.apply(message).await;
    }

    pub async fn apply_scrollback_batch(&self, messages: Vec<crate::widgets::ScrollbackMsg>) {
        self.scrollback_actor.apply_batch(messages).await;
    }

    /// Handle a prompt outcome. Returns Some(text) on submit.
    pub async fn handle_prompt_outcome(&self, outcome: PromptOutcome) -> Option<String> {
        match outcome {
            PromptOutcome::Submitted(text) => {
                let timestamp = crate::clock::unix_timestamp_seconds();
                let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                    content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                    timestamp,
                });
                let (accepted, acknowledged) = tokio::sync::oneshot::channel();
                if self
                    .submission_tx
                    .send((vec![user_msg], accepted))
                    .await
                    .is_err()
                {
                    return None;
                }
                let _ = acknowledged.await;
                Some(text)
            }
            PromptOutcome::Edited | PromptOutcome::Ignored => None,
        }
    }

    /// Spawn the renderer task. Owns the spawned task via JoinHandle.
    pub fn spawn_renderer(
        &self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::watch::Sender<bool>,
    ) {
        let renderer = EventRenderer::with_live_actors(
            self.scrollback_actor.clone(),
            self.status_actor.clone(),
            false,
        );
        let rx = self.bus.subscribe();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: App — drives the renderer to completion.
        let handle = tokio::spawn(async move { renderer.run(rx, shutdown_rx).await });
        (handle, shutdown_tx)
    }

    pub fn status_snapshot(&self) -> StatusBar {
        self.status_actor.snapshot()
    }

    pub fn status_model_snapshot(&self) -> StatusSnapshot {
        self.status_actor.model_snapshot()
    }

    pub fn model_snapshot(&self) -> TuiSnapshot {
        TuiSnapshot {
            ui: self.ui.snapshot(),
            feed: self.feed_model_snapshot(),
            prompt: self.prompt.model_snapshot(),
            status: self.status_model_snapshot(),
        }
    }

    pub fn scrollback_snapshot(&self) -> Scrollback {
        self.scrollback_actor.snapshot()
    }

    pub async fn flush_session(&self) {
        self.session_actor.flush().await;
    }

    pub fn session_snapshot(&self) -> SessionSnapshot {
        self.session_actor.snapshot()
    }

    /// Read the renderer-independent feed model. New projections and
    /// scenario assertions should prefer this API over the compatibility
    /// widget snapshot.
    pub fn feed_model_snapshot(&self) -> FeedSnapshot {
        self.scrollback_actor.model_snapshot()
    }

    /// Build the immutable declarative view description from actor snapshots.
    /// Renderers consume this projection; they do not inspect ownership state
    /// through ad-hoc mutable fields.
    pub fn view_tree(&self) -> Element {
        self.view_document().root
    }

    /// Build the complete renderer-neutral document for one frame. The
    /// document retains both composition (`root`) and component ownership
    /// metadata; callers that only need the legacy element tree can use
    /// `view_tree`.
    pub fn view_document(&self) -> ViewDocument {
        Self::view_document_from_model(&self.model_snapshot())
    }

    pub fn view_tree_from_model(model: &TuiSnapshot) -> Element {
        Self::view_document_from_model(model).root
    }

    pub fn view_document_from_model(model: &TuiSnapshot) -> ViewDocument {
        chat_document_with_props(ViewProps {
            chat: ChatViewProps {
                welcome_visible: model.ui.show_welcome,
                shortcuts_visible: model.ui.shortcuts_open,
                command_palette_visible: model.ui.command_palette_open,
                // The settled small-screen hint is ambient: after the first
                // completed turn it remains below the feed, matching Grok's
                // one-shot tip promotion. Terminal-size gating belongs to
                // the renderer because this projection is size-independent.
                compact_mode_hint_visible: matches!(model.status.state, Status::Ready)
                    && !model.feed.is_empty(),
            },
            header: HeaderViewProps {
                meter: model.status.header_meter(),
                theme: model.status.theme,
            },
            feed: model.feed.clone(),
            prompt: model.prompt.clone(),
            status: model.status.clone(),
            ui: model.ui.clone(),
        })
    }

    pub fn header_view_props(&self) -> HeaderViewProps {
        let status = self.status_snapshot();
        HeaderViewProps {
            meter: status.header_meter(),
            theme: status.theme(),
        }
    }

    /// Lay out the widgets and render them into the given area using `f`.
    pub fn render<F: FnMut(Rect, &mut Buffer)>(&self, area: Rect, mut f: F) {
        let model = self.model_snapshot();
        let layout = chat_layout_with_prompt_height(area, model.prompt.render_height());
        let sb = Scrollback::from_model_snapshot(model.feed);
        let content_rows = sb.measured_content_rows(layout.scrollback, area.height);
        let anchor_row = sb.measured_anchor_row(layout.scrollback, area.height);
        let _ = self
            .scrollback_actor
            .try_apply(crate::widgets::ScrollbackMsg::LayoutMeasured {
                content_rows,
                viewport_rows: layout.scrollback.height as usize,
                anchor_row,
            });
        let mut buf = Buffer::empty(area);
        sb.render_with_terminal_height(layout.scrollback, area.height, &mut buf);
        f(layout.prompt, &mut buf);
        f(layout.status, &mut buf);
    }
}

#[cfg(test)]
mod tests {
    use super::{PromptActor, UiActor, UiMsg, UiState};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use runie_core::events::EventBus;
    use runie_core::types::AgentEvent;

    #[test]
    fn ui_reducer_owns_welcome_and_shortcut_transitions() {
        let initial = UiState::with_welcome();
        assert!(initial.show_welcome);
        assert!(!initial.shortcuts_open);
        let open = initial.clone().update(UiMsg::ToggleShortcuts);
        assert!(open.shortcuts_open);
        let palette = initial.update(UiMsg::ToggleCommandPalette);
        assert!(palette.command_palette_open);
        let activated = palette
            .update(UiMsg::CommandPaletteChar('n'))
            .update(UiMsg::ActivateCommandPalette);
        assert_eq!(
            activated.last_palette_command.as_deref(),
            Some("New Session")
        );
        assert!(!activated.command_palette_open);
        let hidden = open.update(UiMsg::HideWelcome);
        assert!(!hidden.show_welcome);
        assert!(hidden.shortcuts_open);
        assert_eq!(hidden.update(UiMsg::Reset), UiState::new());
    }

    #[test]
    fn view_document_preserves_declarative_composition_and_ownership() {
        let model = super::TuiSnapshot {
            ui: UiState::new(),
            feed: runie_tui_model::FeedState::default().snapshot(),
            prompt: super::PromptWidget::new().model_snapshot(),
            status: super::StatusBar::new().model_snapshot(),
        };
        let document = super::App::view_document_from_model(&model);
        assert_eq!(document.root.slots().count(), 5);
        assert_eq!(
            document.components.len(),
            crate::view::CHAT_COMPONENTS.len()
        );
        assert_eq!(
            crate::view::component(crate::view::Slot::Scrollback).owner,
            crate::view::StateOwner::ScrollbackActor
        );
    }

    #[tokio::test]
    async fn ui_actor_keeps_welcome_disabled_after_reset() {
        let bus = EventBus::new();
        let actor = UiActor::new(&bus);
        assert!(!actor.snapshot().show_welcome);
        bus.publish(AgentEvent::Reset);
        for _ in 0..4 {
            tokio::task::yield_now().await;
            if !actor.snapshot().show_welcome {
                return;
            }
        }
        panic!("UiActor enabled the removed welcome surface");
    }

    #[tokio::test]
    async fn ui_actor_publishes_palette_activation_command() {
        let bus = EventBus::new();
        let actor = UiActor::new(&bus);
        let mut commands = actor.subscribe_commands();
        actor.send(UiMsg::ToggleCommandPalette).await;
        actor.send(UiMsg::CommandPaletteChar('n')).await;
        actor.send(UiMsg::ActivateCommandPalette).await;
        assert_eq!(
            commands.recv().await.unwrap(),
            super::UiCommand::ActivatePaletteEntry(super::PaletteAction::NewSession)
        );
    }

    #[test]
    fn palette_registry_maps_every_visible_entry_to_a_typed_action() {
        assert!(super::PaletteAction::labels()
            .iter()
            .all(|label| super::palette_action_for(label).is_some()));
        assert!(super::palette_action_for("unknown command").is_none());
    }

    #[tokio::test]
    async fn prompt_actor_reacts_to_reset_events() {
        let bus = EventBus::new();
        let actor = PromptActor::new(&bus);
        actor
            .handle_key(KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            })
            .await;
        assert!(!actor.snapshot().is_empty());
        bus.publish(AgentEvent::Reset);
        for _ in 0..4 {
            tokio::task::yield_now().await;
            if actor.snapshot().is_empty() {
                return;
            }
        }
        panic!("PromptActor did not apply the bus reset event");
    }

    #[tokio::test]
    async fn prompt_actor_projects_theme_events_into_prompt_view() {
        let bus = EventBus::new();
        let actor = PromptActor::new(&bus);
        bus.publish(AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::GrokDay,
        });
        for _ in 0..4 {
            tokio::task::yield_now().await;
            let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            });
            ratatui::widgets::Widget::render(
                actor.snapshot(),
                ratatui::layout::Rect {
                    x: 0,
                    y: 0,
                    width: 30,
                    height: 3,
                },
                &mut buffer,
            );
            if buffer
                .cell((2, 1))
                .is_some_and(|cell| cell.fg == ratatui::style::Color::Rgb(38, 38, 38))
            {
                return;
            }
        }
        panic!("PromptActor did not project the theme event");
    }

    #[tokio::test]
    async fn prompt_actor_projects_terminal_native_theme_into_reset_colors() {
        let bus = EventBus::new();
        let actor = PromptActor::new(&bus);
        bus.publish(AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::TerminalNative,
        });
        for _ in 0..4 {
            tokio::task::yield_now().await;
            let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            });
            ratatui::widgets::Widget::render(
                actor.snapshot(),
                ratatui::layout::Rect {
                    x: 0,
                    y: 0,
                    width: 30,
                    height: 3,
                },
                &mut buffer,
            );
            if buffer
                .cell((2, 1))
                .is_some_and(|cell| cell.fg == ratatui::style::Color::Reset)
            {
                return;
            }
        }
        panic!("PromptActor did not project terminal-native theme");
    }

    #[tokio::test]
    async fn prompt_reset_preserves_actor_owned_theme() {
        let bus = EventBus::new();
        let actor = PromptActor::new(&bus);
        bus.publish(AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::RosePineMoon,
        });
        for _ in 0..4 {
            tokio::task::yield_now().await;
            if actor.model_snapshot().theme == runie_core::types::ThemeKind::RosePineMoon {
                break;
            }
        }
        bus.publish(AgentEvent::Reset);
        for _ in 0..4 {
            tokio::task::yield_now().await;
            if actor.model_snapshot().theme == runie_core::types::ThemeKind::RosePineMoon {
                return;
            }
        }
        panic!("PromptActor reset discarded the actor-owned theme");
    }

    #[tokio::test]
    async fn prompt_reset_preserves_actor_owned_model_caption() {
        let bus = EventBus::new();
        let actor = PromptActor::new(&bus);
        actor.set_model_caption("custom-model (high)".into()).await;
        bus.publish(AgentEvent::Reset);
        for _ in 0..4 {
            tokio::task::yield_now().await;
            if actor.model_snapshot().model_caption == "custom-model (high)" {
                return;
            }
        }
        panic!("PromptActor reset discarded the actor-owned model caption");
    }
}
