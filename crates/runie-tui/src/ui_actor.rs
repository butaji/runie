//! UiActor — owns `AppState` and is the sole state mutator.
//!
//! The actor subscribes to the shared `EventBus<Event>`, applies every event to
//! `AppState`, sends fresh `Snapshot`s to the render task via a `watch` channel,
//! and triggers side-effects (agent spawns, clipboard, etc.) without blocking.

use std::collections::HashMap;
use std::time::Duration;

use runie_agent::AgentCommand;
use runie_agent::truncate::TruncationPolicy;
use runie_core::actors::turn::RactorTurnHandle;
use runie_core::actors::RactorInputHandle;
use runie_core::bus::{EventBus, Receiver};
use runie_core::commands::{DialogKind, DialogState};
use runie_core::update::dialog::handle_form_dialog;
use runie_core::{AppState, Event, Snapshot};

use crate::effects::EffectCommand;
use crate::pace::PacedRenderer;
use crate::terminal::caps::TermCaps;

pub use crate::ui_actor_agent_handles::{AgentHandleBox, AgentActorHandle, LeaderAgentActorHandle};

const ANIM_MS: u64 = 100;

/// Actor that owns the application state.
pub struct UiActor {
    pub(crate) state: AppState,
    /// UiActor creates its own watch channel for snapshots so the render task can
    /// receive frames. Call `take_render_rx()` after construction to hand the
    /// receiver to the render task.
    render_tx: tokio::sync::watch::Sender<Snapshot>,
    render_rx: Option<tokio::sync::watch::Receiver<Snapshot>>,
    agent_handle: AgentHandleBox,
    kb_tx: tokio::sync::watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    caps: TermCaps,
    pub(crate) paced: PacedRenderer,
    /// Previous input text (snapshot before last InputChanged application).
    /// Used to detect autocomplete trigger characters.
    prev_input: String,
    /// Previous cursor position (snapshot before last InputChanged application).
    prev_cursor_pos: usize,
    /// Pending submit content captured before sending InputMsg::Submit.
    /// Dispatched after InputChanged is applied so state is clean.
    pending_submit: Option<String>,
    /// Tracks whether a turn was active (agent was spawned) in the previous turn cycle.
    /// Set when an agent is spawned; cleared when `TurnCompleted`/`Abort` resets the state.
    /// Used by the guard to block a `TurnStarted` that arrives after `Done` clears
    /// `turn_active` but before the guard has settled for the new cycle.
    turn_was_active: bool,
    /// True when the pending turn was started from a delivered (queued) message,
    /// not a fresh user submit. When true, UiActor skips calling submit_user_message
    /// for TurnStarted because the content was already delivered via FollowUpDelivered.
    pending_queued_turn: bool,
    /// Turn actor handle for draining the queue after a turn completes.
    /// Stored here so UiActor can call run_if_queued after Done is processed.
    turn_handle: Option<RactorTurnHandle>,
    /// Input actor handle for sending InputMsg to InputActor.
    /// Stored here so UiActor can route input events without going through actor_handles.
    input_handle: Option<RactorInputHandle>,
    /// Placeholder receiver stored when UiActor is created with `with_external_bus_rx`.
    /// Consumed by `run_with_external_rx`.
    _bus_rx: Option<Receiver<Event>>,
}

impl UiActor {
    /// Create a new `UiActor` with an mpsc-backed agent handle.
    /// UiActor creates its own watch channel for snapshots; call `take_render_rx()`
    /// to hand the receiver to the render task.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: AppState,
        agent_handle: AgentActorHandle,
        turn_handle: RactorTurnHandle,
        input_handle: RactorInputHandle,
        kb_tx: tokio::sync::watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: tokio::sync::oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        Self::with_agent_handle(
            state,
            AgentHandleBox::Actor(agent_handle),
            Some(turn_handle),
            Some(input_handle),
            kb_tx,
            bus,
            shutdown_tx,
            caps,
        )
    }

    /// Create a new `UiActor` with a pre-created bus receiver.
    ///
    /// Use this when you need UiActor to subscribe to the bus BEFORE actors emit
    /// initial facts (e.g. `ConfigLoaded`). Create the bus, subscribe, pass the
    /// receiver here, then call `leader.start_with_bus()`. UiActor will receive
    /// all initial facts. Call `set_agent_handle()` after `start_with_bus()` returns.
    #[allow(clippy::too_many_arguments)]
    pub fn with_external_bus_rx(
        mut state: AppState,
        bus_rx: Receiver<Event>,
        turn_handle: RactorTurnHandle,
        input_handle: RactorInputHandle,
        kb_tx: tokio::sync::watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: tokio::sync::oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        let (render_tx, render_rx) = tokio::sync::watch::channel(state.snapshot());
        let prev_input = state.input().input.clone();
        let prev_cursor_pos = state.input().cursor_pos;
        Self {
            state,
            render_tx,
            render_rx: Some(render_rx),
            agent_handle: AgentHandleBox::Leader(LeaderAgentActorHandle::new_noop()),
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
            paced: PacedRenderer::new(),
            prev_input,
            prev_cursor_pos,
            pending_submit: None,
            turn_was_active: false,
            pending_queued_turn: false,
            turn_handle: Some(turn_handle),
            input_handle: Some(input_handle),
            // Store the pre-created receiver for run_with_external_rx
            _bus_rx: Some(bus_rx),
        }
    }

    /// Create a new `UiActor` with a generic agent handle.
    /// UiActor creates its own watch channel for snapshots; call `take_render_rx()`
    /// to hand the receiver to the render task.
    #[allow(clippy::too_many_arguments)]
    pub fn with_agent_handle(
        mut state: AppState,
        agent_handle: AgentHandleBox,
        turn_handle: Option<RactorTurnHandle>,
        input_handle: Option<RactorInputHandle>,
        kb_tx: tokio::sync::watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: tokio::sync::oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        let (render_tx, render_rx) = tokio::sync::watch::channel(state.snapshot());
        let prev_input = state.input().input.clone();
        let prev_cursor_pos = state.input().cursor_pos;
        Self {
            state,
            render_tx,
            render_rx: Some(render_rx),
            agent_handle,
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
            paced: PacedRenderer::new(),
            prev_input,
            prev_cursor_pos,
            pending_submit: None,
            turn_was_active: false,
            pending_queued_turn: false,
            turn_handle,
            input_handle,
            _bus_rx: None,
        }
    }

    /// Replace the agent handle after construction.
    /// Use this when UiActor is created before `Leader::start_with_bus()` returns
    /// (so the real agent handle is not yet available). Call this after
    /// `leader.start_with_bus()` to install the real handle.
    pub fn set_agent_handle(&mut self, handle: AgentHandleBox) {
        self.agent_handle = handle;
    }

    /// Run the actor with a pre-created bus receiver.
    ///
    /// Use this when you need to subscribe to the bus BEFORE `Leader::start_with_bus()`
    /// returns (so that UiActor receives initial facts like `ConfigLoaded`).
    /// Create the bus, subscribe UiActor, call `start_with_bus()`, then call this method.
    pub async fn run_with_external_rx(
        mut self,
        submit_rx: tokio::sync::mpsc::Receiver<Event>,
    ) {
        let rx = self._bus_rx.take().expect(
            "run_with_external_rx requires UiActor created with with_external_bus_rx",
        );
        self.run(rx, submit_rx).await;
    }

    /// Take the snapshot channel receiver, transferring ownership to the render task.
    /// Must be called exactly once, after construction and before `run()`.
    pub fn take_render_rx(&mut self) -> tokio::sync::watch::Receiver<Snapshot> {
        self.render_rx.take().expect("render_rx already taken")
    }

    /// Run the actor until a quit event is processed.
    pub async fn run(
        mut self,
        mut rx: Receiver<Event>,
        mut submit_rx: tokio::sync::mpsc::Receiver<Event>,
    ) {
        let (effect_tx, effect_rx) = tokio::sync::mpsc::channel::<Event>(16);
        Self::spawn_effect_forwarder(self.bus.clone(), effect_rx);

        // Drain all buffered bootstrap events before sending the first snapshot.
        // Events from `Leader::start_with_bus()` (ConfigLoaded, TrustLoaded, etc.)
        // are sent before UiActor's run() starts. Without draining, the first
        // snapshot is rendered with empty/default state, causing a flash once
        // those events arrive and are applied.
        loop {
            match rx.try_recv() {
                Ok(evt) => {
                    if self.handle_event_inner(evt, effect_tx.clone()).await {
                        // Quit event — still publish a final snapshot before exiting.
                        self.publish_snapshot();
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));
        self.state.ensure_fresh();
        let snap = self.build_paced_snapshot();
        let _ = self.render_tx.send(snap);

        loop {
            tokio::select! {
                Ok(evt) = rx.recv() => {
                    if self.handle_event_inner(evt, effect_tx.clone()).await {
                        break;
                    }
                    // Drain any events already queued (e.g. streaming response
                    // deltas) and apply them in one batch, then publish a single
                    // snapshot for the whole burst instead of one per token.
                    while let Ok(evt) = rx.try_recv() {
                        if self.handle_event_inner(evt, effect_tx.clone()).await {
                            // Quit: break out of while loop to publish final snapshot.
                            break;
                        }
                    }
                    self.publish_snapshot();
                }
                Some(evt) = submit_rx.recv() => {
                    if self.handle_event_inner(evt, effect_tx.clone()).await {
                        break;
                    }
                    self.publish_snapshot();
                }
                _ = anim.tick() => {
                    self.state.tick_animation();
                    self.paced.tick();
                    self.publish_snapshot();
                }
            }
        }

        self.publish_snapshot();
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    fn spawn_effect_forwarder(bus: EventBus<Event>, mut rx: tokio::sync::mpsc::Receiver<Event>) {
        tokio::spawn(async move {
            while let Some(evt) = rx.recv().await {
                bus.publish(evt);
            }
        });
    }

    /// Handle a single event and publish a fresh snapshot.
    /// Returns `true` when the actor should shut down.
    #[cfg(test)]
    pub(crate) async fn handle_event(&mut self, evt: Event, effect_tx: tokio::sync::mpsc::Sender<Event>) -> bool {
        let quit = self.handle_event_inner(evt, effect_tx).await;
        self.publish_snapshot();
        quit
    }

    /// Return whether an agent turn is in flight.
    /// True when a turn is currently active (`turn_active`) or was active in the
    /// previous cycle (`turn_was_active`). After `Done` clears `turn_active`, the
    /// guard keeps `turn_was_active = true` until `TurnCompleted`/`Abort`.
    #[cfg(test)]
    pub(crate) fn agent_running(&self) -> bool {
        self.state.agent_state().turn_active || self.turn_was_active
    }

    /// Handle a single event without publishing. Returns `true` when the actor
    /// should shut down.
    pub(crate) async fn handle_event_inner(&mut self, evt: Event, effect_tx: tokio::sync::mpsc::Sender<Event>) -> bool {
        // Priority quit handling — Ctrl+C / Ctrl+Q always exit, even during active turns.
        if matches!(&evt, Event::Quit | Event::ForceQuit) {
            return true;
        }
        // Capture whether the turn was already active BEFORE apply_event runs.
        // apply_event is called inside handle_input_event, so this must be at the
        // very top to capture the pre-event state.
        let prev_turn_active = self.state.agent_state().turn_active;
        let was_config_loaded = matches!(&evt, Event::ConfigLoaded { .. });
        // Track whether `Done` was just applied so `agent_running()` stays true until
        // `TurnCompleted`/`Abort`. Done clears `turn_active` but must not clear the guard.
        self.handle_input_event(&evt).await;

        if !matches!(&evt, Event::InputChanged { .. }) {
            self.update_paced_renderer(&evt);
            self.dispatch_effect(&evt, effect_tx.clone()).await;
        }
        if *self.state.should_quit_mut() {
            return true;
        }
        if was_config_loaded {
            let _ = self.kb_tx.send(self.state.config().keybindings().clone());
        }

        // Track pending queued turn: set when FollowUpDelivered is applied.
        // The content was already delivered to the session via FollowUpDelivered;
        // UiActor should NOT call submit_user_message again (which would emit
        // a duplicate UserMessageSubmitted).
        if matches!(&evt, Event::FollowUpDelivered { .. } | Event::SteeringDelivered { .. }) {
            self.pending_queued_turn = true;
        }


        if let Event::TurnStarted { request_id, content, .. } = &evt {
            // Guard: prevent duplicate agent spawns if TurnStarted arrives multiple times.
            // prev_turn_active was captured at the top of this function, BEFORE
            // apply_event (inside handle_input_event) updated the projection.
            // turn_was_active is set when an agent was spawned in the previous turn cycle.
            if !prev_turn_active && !self.turn_was_active {
                self.turn_was_active = true;
                let provider = self.state.config().current_provider.clone();
                let model = self.state.config().current_model.clone();
                let cmd = AgentCommand {
                    content: content.clone(),
                    id: request_id.clone(),
                    provider,
                    model,
                    thinking_level: self.state.config().thinking_level,
                    read_only: false,
                    skills_context: String::new(),
                    system_prompt: String::new(),
                    truncation: TruncationPolicy::default(),
                    cancellation_token: tokio_util::sync::CancellationToken::new(),
                };
                self.agent_handle.run(cmd).await;
            }
            // Clear the queued-turn flag now that the turn has started.
            // (submit_user_message was already called for queued turns by TurnActor.)
            self.pending_queued_turn = false;
        }

        // Clear agent_running and drain the queue when the turn fully completes
        // (TurnCompleted), errors (TurnErrored), or is explicitly aborted (Abort).
        //
        // We do NOT clear agent_running on Done — Done is emitted by the agent actor
        // before the turn state is fully finalized. Clearing here would allow a
        // TurnStarted from run_if_queued (also called on Done) to bypass the guard
        // and spawn a second agent, causing doubled output on the same stream.
        // The real guard-clear happens on TurnCompleted / TurnErrored / Abort.
        //
        // FIX: /new aborts the turn and clears the queue. This is called from both
        // handle_event_inner (for Abort from event bus) and dispatch_submit_content
        // (for Abort from CommandResult::Events from /new handler).
        if matches!(&evt, Event::TurnCompleted | Event::TurnErrored { .. } | Event::Abort) {
            let is_abort = matches!(&evt, Event::Abort);
            self.clear_turn_state(is_abort).await;
        }


        false
    }

    /// Route input events through InputActor instead of applying directly.
    /// Route input events through `route_to_input_actor` (the canonical mapping).
    /// UiActor-specific cases (permission dialog y/n/a, Submit, InputChanged) are
    /// handled separately; everything else is routed via the shared helper.
    ///
    /// UiActor must NEVER mutate `AppState.input` directly — only through `apply_event`.
    async fn handle_input_event(&mut self, evt: &Event) {
        // Permission-dialog guard: suppress navigation/editing keys while dialog is open.
        // y/n/a are handled in the special case below.
        if self.state.permission_request_opt().is_some()
            && is_navigation_or_editing_event(evt)
        {
            return;
        }

        // Canonical routing via the shared helper (one place to maintain the mapping).
        if let Some(ref handle) = self.input_handle {
            if crate::input_mapping::route_to_input_actor(handle, evt).await {
                return;
            }
        }

        // UiActor-specific event handling (not routed to InputActor).
        match evt {
            Event::Input(c) => {
                // Intercept y/n/a keys when a permission dialog is open.
                if let Some(req) = self.state.permission_request_opt() {
                    match c.to_ascii_lowercase() {
                        'y' | 'n' | 'a' => {
                            let action = match c.to_ascii_lowercase() {
                                'y' | 'a' => runie_core::permissions::PermissionAction::Allow,
                                'n' => runie_core::permissions::PermissionAction::Deny,
                                _ => return,
                            };
                            if let Some(handles) = self.state.actor_handles() {
                                handles
                                    .permission
                                    .try_resolve_permission(req.request_id.clone(), action);
                            }
                            *self.state.permission_request_mut() = None;
                            self.state.view_mut().dirty = true;
                        }
                        _ => {}
                    }
                }
                // Non-permission Input events would have been routed above.
            }
            Event::Submit => {
                self.handle_submit_event().await;
            }
            Event::InputChanged { state } => {
                self.handle_input_changed(state).await;
            }
            _ => {
                self.apply_event(evt.clone());
            }
        }
    }

    /// Handle the Submit event by capturing content and sending to InputActor.
    /// If a form dialog is open, route to the form handler instead.
    async fn handle_submit_event(&mut self) {
        // If a form dialog is open, route Enter to the form instead of the input box.
        // This is the path that makes /save, /load, /compact etc. submittable.
        if is_form_dialog_open(&self.state) {
            handle_form_dialog(&mut self.state, Event::CommandFormSubmit);
            return;
        }
        let content = self.state.input().input().trim().to_owned();
        self.pending_submit = if content.is_empty() {
            None
        } else {
            Some(content.clone())
        };
        self.send_input_msg(runie_core::actors::InputMsg::Submit { content })
            .await;
    }

    /// Handle InputChanged: route through apply_event so all state mutations
    /// flow through one canonical path, then trigger side effects.
    /// UiActor must NEVER mutate AppState.input directly — only through apply_event.
    async fn handle_input_changed(&mut self, state: &runie_core::InputState) {
        // Capture prev_input BEFORE apply_event changes self.state.input.
        let prev_input = self.prev_input.clone();
        let prev_cursor_pos = self.prev_cursor_pos;
        let new_input = state.input().to_owned();
        let new_cursor_pos = state.cursor_pos;

        // Route through apply_event — the single source of truth for state mutations.
        // UiActor must NOT mutate AppState.input directly.
        self.apply_event(Event::InputChanged {
            state: Box::new((*state).clone()),
        });

        self.detect_autocomplete_trigger(&prev_input, prev_cursor_pos, &new_input, new_cursor_pos);

        if let Some(content) = self.pending_submit.take() {
            self.dispatch_submit_content(content).await;
        }

        self.state.view_mut().dirty = true;
        self.handle_at_trigger();
    }

    /// Update the paced renderer based on the received event.
    fn update_paced_renderer(&mut self, evt: &Event) {
        match evt {
            Event::TextStart { .. } => {
                self.paced = PacedRenderer::new();
            }
            Event::ResponseDelta { content, .. } => {
                self.paced.push(content);
            }
            Event::TurnComplete { .. } | Event::Done { .. } => {
                self.paced.finish();
            }
            _ => {}
        }
    }

    fn apply_event(&mut self, evt: Event) {
        self.state.update(evt);
    }

    /// Dispatch effects via IoActor.
    async fn dispatch_effect(&mut self, evt: &Event, effect_tx: tokio::sync::mpsc::Sender<Event>) {
        if let Some(cmd) = EffectCommand::try_from_event(evt, &mut self.state, &self.caps) {
            // For login validation, handle separately
            if matches!(cmd, EffectCommand::LoginFlowSubmitKey { .. }) {
                let flow = self.state.login_flow().cloned();
                if let Some(f) = flow {
                    let tx = effect_tx.clone();
                    let provider_handle = self
                        .state
                        .actor_handles()
                        .as_ref()
                        .map(|h| h.provider.clone());
                    if let Some(handle) = provider_handle {
                        tokio::spawn(crate::effects::login::run(f.provider, f.key, tx, handle.clone()));
                    }
                }
            } else {
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    cmd.dispatch_async(&state_clone).await;
                });
            }
        }
    }

    /// Build a snapshot with the paced streaming tail applied.
    fn build_paced_snapshot(&mut self) -> Snapshot {
        self.state.ensure_fresh();
        let mut snap = self.state.snapshot();
        // Show the paced display text instead of the raw streaming tail.
        snap.streaming_tail = self.paced.displayed().to_owned();
        snap
    }

    /// Fire-and-forget send to InputActor.
    async fn send_input_msg(&self, msg: runie_core::actors::InputMsg) {
        if let Some(ref handle) = self.input_handle {
            let _ = handle.send_message(msg);
        }
    }

    /// Check if a command is a quit command (matches slash-command semantics).
    fn is_quit_command(content: &str) -> bool {
        matches!(content.trim(), "/q" | "/quit" | "/exit")
    }

    /// Detect autocomplete trigger characters ('@' or '/') typed at end of input.
    /// Opens the command palette or file picker accordingly.
    fn detect_autocomplete_trigger(
        &mut self,
        prev_input: &str,
        _prev_cursor: usize,
        new_input: &str,
        new_cursor: usize,
    ) {
        // Detect '@' or '/' typed at end of input (not inside existing autocomplete).
        let was_empty_or_space =
            prev_input.is_empty() || prev_input.ends_with(' ') || prev_input.ends_with('\n');

        if was_empty_or_space
            && !new_input.is_empty()
            && new_cursor == new_input.len()
            && self.state.completion().at_suggestions.is_none()
        {
            let last_char = new_input.chars().last().unwrap();
            if last_char == '@' {
                // Open file picker.
                let (input_text, cursor) = (new_input.to_owned(), new_cursor);
                self.state.input_mut().file_picker_backup =
                    Some((input_text, cursor, cursor, false));
                runie_core::update::dialog::open_at_file_picker_all(&mut self.state);
                self.state.view_mut().dirty = true;
            } else if last_char == '/' && !Self::is_quit_command(new_input) {
                // Open command palette.
                self.state.input_mut().input = String::new();
                self.state.input_mut().cursor_pos = 0;
                runie_core::update::dialog::open_command_palette_with_filter(&mut self.state, "");
                self.state.view_mut().dirty = true;
            }
        }
    }

    /// Handle autocomplete trigger at current cursor position.
    fn handle_at_trigger(&mut self) {
        let input = self.state.input();
        let is_empty_or_space =
            input.input.is_empty() || input.input.ends_with(' ') || input.input.ends_with('\n');
        if is_empty_or_space
            || self.state.completion().at_suggestions.is_some()
            || input.input.ends_with('@')
        {
            return;
        }

        let last_char = input.input.chars().last().unwrap();
        if last_char == '@' && input.cursor_pos == input.input.len() {
            // File picker: already opened in detect_autocomplete_trigger.
            return;
        }

        if last_char == '/' && !Self::is_quit_command(&input.input) {
            // Command palette: already opened in detect_autocomplete_trigger.
        }
    }

    /// Clear agent-running flag and queue.
    ///
    /// Used for both `Event::Abort` (from /new or event bus) and
    /// `Event::TurnCompleted`/`TurnErrored` (from turn lifecycle).
    ///
    /// For Abort: clears the queue so a new session starts clean.
    /// For TurnCompleted: delivers queued messages and starts the next turn.
    async fn clear_turn_state(&mut self, is_abort: bool) {
        // Derive agent_running from turn_active (updated by apply_event for
        // TurnCompleted/TurnErrored, or cleared directly for Abort).
        self.state.agent_state_mut().turn_active = false;
        self.turn_was_active = false;
        self.pending_queued_turn = false;
        if let Some(ref turn_handle) = self.turn_handle {
            if is_abort {
                // Abort: clear the queue so a new session starts clean.
                turn_handle.send(runie_core::actors::TurnMsg::ClearQueues).await;
            } else {
                // TurnCompleted: deliver queued messages and start the next turn.
                // Uses ractor RPC so TurnActor emits FollowUpDelivered/SteeringDelivered
                // before this function returns — no polling, no late-arriving-event race.
                let steering_mode = self.state.config().steering_mode;
                let follow_up_mode = self.state.config().follow_up_mode;
                use runie_core::actors::turn::DeliverQueuedRpcResult as DQR;
                let deliver_result = turn_handle
                    .deliver_queued(steering_mode, follow_up_mode)
                    .await;
                match deliver_result {
                    DQR::Delivered(Some(_)) => tracing::debug!("Queued turn delivered"),
                    DQR::Delivered(None) => tracing::debug!("No queued turn to deliver"),
                    DQR::SenderError => tracing::warn!("DeliverQueued RPC sender error"),
                    DQR::ActorError(e) => tracing::warn!("DeliverQueued RPC error: {}", e),
                }
                self.agent_handle.run_if_queued(turn_handle).await;
            }
        }
    }

    /// Dispatch submit content (slash command, form submission, steering, or user message).
    pub(crate) async fn dispatch_submit_content(&mut self, content: String) {
        // If a form dialog is open and chat input is empty, this is a form submission
        // (the form field content lives in the panel, not the chat input).
        // Route through handle_form_dialog so Enter on the submit button works.
        if self.state.open_dialog().is_some() && content.is_empty() {
            let form_handled = self.maybe_submit_form();
            if form_handled {
                // Form was submitted → dialog is now closed, command dispatched.
                self.state.view_mut().scroll = 0;
                self.state.view_mut().dirty = true;
                return;
            }
            // Not a form panel — fall through to close dialog and handle as slash command.
        }
        // Close any open dialog (e.g., command palette) before executing the command.
        *self.state.open_dialog_mut() = None;
        // Slash command handling.
        if let Some(result) = self.state.handle_slash(&content) {
            // Extract Abort/ClearQueues from CommandResult::Events before applying,
            // so UiActor flags are cleared even though handle_event_inner is bypassed.
            let has_abort = matches!(&result, runie_core::commands::CommandResult::Events(evts) if evts.iter().any(|e| matches!(e, Event::Abort)));
            self.state.apply_command_result(result);
            if has_abort {
                self.clear_turn_state(true).await;
            }
            self.state.view_mut().scroll = 0;
            self.state.view_mut().dirty = true;
            return;
        }
        // Steering (follow-up during active turn): route through TurnActor to
        // maintain authoritative queue state. When the turn completes,
        // UiActor::handle_event_inner calls DeliverQueued + RunIfQueued to start
        // the queued turn.
        if self.state.agent_state().turn_active {
            self.state.queue_steering_and_update_history(content);
            return;
        }
        // Normal user message submission.
        self.state.submit_user_message_and_update_history(content);
    }

    /// If a form panel is open, emit CommandFormSubmit and return true.
    /// Returns `false` if no form panel is open, so the caller knows to use the
    /// fallback behavior (close dialog and handle as slash command).
    fn maybe_submit_form(&mut self) -> bool {
        // Quick check: is a dialog open and is it a form?
        if self.state.open_dialog().is_none() {
            return false;
        }
        // handle_form_dialog handles Generic dialogs with form panels.
        // For non-form dialogs (command palette, etc.) it does nothing.
        // If the form was submitted, the dialog is now closed.
        // If not (e.g. validation failure), the dialog is still open.
        let was_open = self.state.open_dialog().is_some();
        handle_form_dialog(&mut self.state, Event::CommandFormSubmit);
        // If dialog was already closed by handle_form_dialog, return true (handled).
        // If it was a form that kept open (validation), also return true.
        // Only return false if no form dialog was open (non-form dialog path).
        if !was_open || self.state.open_dialog().is_some() {
            // Dialog is still open → form kept it open (not submitted).
            // Return false so the caller closes it as a non-form dialog.
            return false;
        }
        // Dialog was closed by handle_form_dialog → form was submitted.
        true
    }

    fn publish_snapshot(&mut self) {
        let snap = self.build_paced_snapshot();
        let _ = self.render_tx.send(snap);
    }
}

/// Returns `true` for events that should be silently consumed (no-op) while
/// a permission dialog is open. These keys must NOT deny the request and must
/// NOT be routed to the input box.
///
/// This intentionally does NOT include `Event::Input` — those are handled
/// separately in `handle_input_event` to resolve the permission.
fn is_navigation_or_editing_event(evt: &Event) -> bool {
    matches!(
        evt,
        Event::Escape
            | Event::Backspace
            | Event::Newline
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::Undo
            | Event::Redo
            | Event::Paste(_)
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom
            | Event::Submit
            | Event::MouseScrollUp
            | Event::MouseScrollDown
            | Event::MouseClick { .. }
            | Event::MouseMove { .. }
            | Event::TerminalSize { .. }
    )
}

/// Returns `true` if a Generic dialog with a form panel is currently open
/// and no login flow is active.
///
/// Used to route Enter/Tab to the command form handler instead of the input box.
/// Login flow dialogs also use Generic + Form panels but have their own submission
/// mechanism (button actions emit `Event::Save`), so they are excluded.
fn is_form_dialog_open(state: &AppState) -> bool {
    // Exclude login flow: it uses Generic+Form panels but its submit button
    // emits Event::Save (handled by login_flow_event), not CommandFormSubmit.
    if state.login_flow().is_some() {
        return false;
    }
    state.open_dialog().is_some_and(|d| {
        if let DialogState::Active {
            kind: DialogKind::Generic,
            panels,
        } = d
        {
            panels.current().is_some_and(|p| p.is_form())
        } else {
            false
        }
    })
}
