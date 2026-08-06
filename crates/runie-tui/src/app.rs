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
pub use crate::widgets::PaletteAction;
use crate::widgets::{PromptOutcome, PromptWidget, Scrollback, StatusBar};

#[derive(Debug)]
pub enum AppExit {
    Quit,
    Error(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiState {
    pub show_welcome: bool,
    pub shortcuts_open: bool,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub command_palette_index: usize,
    pub last_palette_command: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMsg {
    HideWelcome,
    ToggleShortcuts,
    ToggleCommandPalette,
    CommandPaletteChar(char),
    CommandPaletteBackspace,
    CommandPaletteMove(isize),
    ActivateCommandPalette,
    Reset,
}

/// Commands emitted by the UI actor after a pure palette reduction. Consumers
/// execute these commands through their own actor/event boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    ActivatePaletteEntry(PaletteAction),
}

fn palette_command_for(state: &UiState, message: UiMsg) -> Option<UiCommand> {
    if !matches!(message, UiMsg::ActivateCommandPalette) {
        return None;
    }
    crate::widgets::CommandPaletteWidget::selected_entry(
        &state.command_palette_query,
        state.command_palette_index,
    )
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

impl UiState {
    pub fn new() -> Self {
        Self {
            show_welcome: false,
            shortcuts_open: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_index: 0,
            last_palette_command: None,
        }
    }

    pub fn with_welcome() -> Self {
        Self {
            show_welcome: true,
            shortcuts_open: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_index: 0,
            last_palette_command: None,
        }
    }

    pub fn update(mut self, msg: UiMsg) -> Self {
        match msg {
            UiMsg::HideWelcome => self.show_welcome = false,
            UiMsg::ToggleShortcuts => self.shortcuts_open = !self.shortcuts_open,
            UiMsg::ToggleCommandPalette => {
                self.command_palette_open = !self.command_palette_open;
                if !self.command_palette_open {
                    self.command_palette_query.clear();
                    self.command_palette_index = 0;
                }
            }
            UiMsg::CommandPaletteChar(ch) => self.command_palette_query.push(ch),
            UiMsg::CommandPaletteBackspace => {
                self.command_palette_query.pop();
            }
            UiMsg::CommandPaletteMove(delta) => {
                let count =
                    crate::widgets::CommandPaletteWidget::entry_count(&self.command_palette_query);
                if count > 0 {
                    self.command_palette_index = self
                        .command_palette_index
                        .saturating_add_signed(delta)
                        .min(count - 1);
                } else {
                    self.command_palette_index = 0;
                }
            }
            UiMsg::ActivateCommandPalette => {
                self.last_palette_command = crate::widgets::CommandPaletteWidget::selected_entry(
                    &self.command_palette_query,
                    self.command_palette_index,
                )
                .map(str::to_owned);
                self.command_palette_open = false;
                self.command_palette_query.clear();
                self.command_palette_index = 0;
            }
            UiMsg::Reset => self = Self::new(),
        }
        self
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
                        if matches!(event, Ok(AgentEvent::Reset)) {
                            state = state.update(UiMsg::Reset);
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
        let (applied, acknowledged) = tokio::sync::oneshot::channel();
        if self.tx.send((message, applied)).await.is_ok() {
            let _ = acknowledged.await;
        }
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
}

#[derive(Clone)]
pub struct PromptActor {
    tx: mpsc::Sender<PromptMsg>,
    snapshot: watch::Receiver<PromptWidget>,
    _owner: std::sync::Arc<runie_core::task_owner::TaskOwner>,
}

impl PromptActor {
    pub fn new(bus: &EventBus) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(PromptWidget::new());
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

    async fn unit(&self, message: PromptMsg) {
        let _ = self.tx.send(message).await;
    }

    pub async fn handle_key(&self, key: KeyEvent) -> PromptOutcome {
        let (reply, result) = tokio::sync::oneshot::channel();
        if self.tx.send(PromptMsg::Key(key, reply)).await.is_err() {
            return PromptOutcome::Ignored;
        }
        result.await.unwrap_or(PromptOutcome::Ignored)
    }

    pub async fn clear(&self) {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.unit(PromptMsg::Clear(reply)).await;
        let _ = result.await;
    }

    pub async fn cycle_mode(&self) {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.unit(PromptMsg::CycleMode(reply)).await;
        let _ = result.await;
    }

    pub async fn set_placeholder_visible(&self, visible: bool) {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.unit(PromptMsg::SetPlaceholderVisible(visible, reply))
            .await;
        let _ = result.await;
    }

    pub async fn open_file_search(&self) {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.unit(PromptMsg::OpenFileSearch(reply)).await;
        let _ = result.await;
    }

    pub async fn set_model_caption(&self, caption: String) {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.unit(PromptMsg::SetCaption(caption, reply)).await;
        let _ = result.await;
    }

    pub fn snapshot(&self) -> PromptWidget {
        self.snapshot.borrow().clone()
    }
}

async fn run_prompt_actor(
    mut rx: mpsc::Receiver<PromptMsg>,
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    snapshot_tx: watch::Sender<PromptWidget>,
) {
    let mut prompt = PromptWidget::new();
    loop {
        tokio::select! {
            message = rx.recv() => {
                let Some(message) = message else { break };
                handle_prompt_message(&mut prompt, message);
                let _ = snapshot_tx.send(prompt.clone());
            }
            event = events.recv() => {
                match event {
                    Ok(AgentEvent::Reset) => {
                        prompt = PromptWidget::new();
                        let _ = snapshot_tx.send(prompt.clone());
                    }
                    Ok(AgentEvent::ThemeChanged { theme }) => {
                        prompt.set_theme(theme);
                        let _ = snapshot_tx.send(prompt.clone());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn handle_prompt_message(prompt: &mut PromptWidget, message: PromptMsg) {
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
            prompt.open_file_search();
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
    }
}

pub struct App {
    pub prompt: PromptActor,
    pub status_actor: StatusActor,
    pub scrollback_actor: ScrollbackActor,
    pub loop_actor: LoopActor,
    pub bus: EventBus,
    pub ui: UiActor,
}

impl App {
    pub fn new(loop_actor: LoopActor, bus: EventBus) -> Self {
        let ui = UiActor::new(&bus);
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new_with_bus(&bus),
            scrollback_actor: ScrollbackActor::new_with_bus(&bus),
            loop_actor,
            bus,
            ui,
        }
    }

    pub fn new_with_welcome(loop_actor: LoopActor, bus: EventBus) -> Self {
        let ui = UiActor::new_with_welcome(&bus, true);
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new_with_bus(&bus),
            scrollback_actor: ScrollbackActor::new_with_bus(&bus),
            loop_actor,
            bus,
            ui,
        }
    }

    pub async fn toggle_shortcuts(&self) {
        self.ui.send(UiMsg::ToggleShortcuts).await;
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

    /// Apply a feed update through the actor that owns the rendered snapshot.
    /// The mutex is a compatibility fallback for apps whose renderer is not
    /// running yet.
    pub async fn apply_scrollback(&self, message: crate::widgets::ScrollbackMsg) {
        self.scrollback_actor.apply(message).await;
    }

    pub async fn apply_scrollback_batch(&self, messages: Vec<crate::widgets::ScrollbackMsg>) {
        self.scrollback_actor.apply_batch(messages).await;
    }

    pub async fn refresh_model_caption(&self) {
        let model = self.loop_actor.state_snapshot().model;
        if !model.name.is_empty() {
            self.prompt
                .set_model_caption(format!("{} (high)", model.name))
                .await;
        }
    }

    /// Handle a prompt outcome. Returns Some(text) on submit.
    pub async fn handle_prompt_outcome(&mut self, outcome: PromptOutcome) -> Option<String> {
        match outcome {
            PromptOutcome::Submitted(text) => {
                let timestamp = crate::clock::unix_timestamp_seconds();
                let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                    content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                    timestamp,
                });
                let _ = self
                    .loop_actor
                    .prompt(vec![user_msg], runie_core::types::AgentContext::default())
                    .await;
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
        let renderer = EventRenderer::with_actors(
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

    pub fn scrollback_snapshot(&self) -> Scrollback {
        self.scrollback_actor.snapshot()
    }

    /// Lay out the widgets and render them into the given area using `f`.
    pub fn render<F: FnMut(Rect, &mut Buffer)>(&self, area: Rect, mut f: F) {
        let layout = chat_layout_with_prompt_height(area, self.prompt.snapshot().render_height());
        let mut sb = self.scrollback_snapshot();
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
        let labels = [
            "New Session",
            "New Session in Worktree",
            "Agent Dashboard",
            "Back to Home",
            "Delete This Session",
            "Resume Session",
            "Share Session",
            "Rename Session",
            "Session Info",
            "Compact History",
            "Context Usage",
            "View Plan",
            "Memory",
            "Switch Model",
            "Keyboard Shortcuts",
            "Quit",
        ];
        assert!(labels
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
}
