//! UiActor module — owns `AppState` and is the sole state mutator.
//!
//! Split into focused submodules:
//! - `input.rs` — Input handling, autocomplete detection, form detection
//! - `submit.rs` — Submit content dispatch
//! - `effects.rs` — Effects dispatch
//! - `helpers.rs` — Utility functions

pub mod effects;
pub mod helpers;
pub mod input;
pub mod submit;

pub use crate::ui_actor_agent_handles::{AgentActorHandle, AgentHandleBox, LeaderAgentActorHandle};

use std::collections::HashMap;
use std::time::Duration;

use runie_agent::AgentCommand;
use runie_agent::truncate::TruncationPolicy;
use runie_core::actors::RactorInputHandle;
use runie_core::actors::turn::RactorTurnHandle;
use runie_core::bus::{EventBus, Receiver};
use runie_core::update::dialog::handle_form_dialog;
use runie_core::permissions::PermissionAction;
use runie_core::{AppState, Event, Snapshot};

use crate::channels::EFFECT_FORWARDER_CHANNEL_CAPACITY;
use crate::pace::PacedRenderer;
use crate::terminal::caps::TermCaps;

/// Animation frame rate: 60fps = ~16.67ms per frame.
/// Public for testing.
pub(crate) const ANIM_MS: u64 = 16;

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
    /// Characters routed to the InputActor whose `InputChanged` echo has not
    /// been processed yet. The input projection lags one round-trip behind
    /// real typing; autocomplete trigger checks must include these pending
    /// characters or they read a stale (shorter) input.
    pending_input_chars: Vec<char>,
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
        let state_bus = bus.clone();
        let mut this = Self {
            state,
            render_tx,
            render_rx: Some(render_rx),
            agent_handle: AgentHandleBox::Leader(LeaderAgentActorHandle::new_noop()),
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
            paced: PacedRenderer::new(),
            pending_input_chars: Vec::new(),
            pending_submit: None,
            turn_was_active: false,
            pending_queued_turn: false,
            turn_handle: Some(turn_handle),
            input_handle: Some(input_handle),
            // Store the pre-created receiver for run_with_external_rx
            _bus_rx: Some(bus_rx),
        };
        this.state.set_event_bus(state_bus);
        this
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
        let state_bus = bus.clone();
        let mut this = Self {
            state,
            render_tx,
            render_rx: Some(render_rx),
            agent_handle,
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
            paced: PacedRenderer::new(),
            pending_input_chars: Vec::new(),
            pending_submit: None,
            turn_was_active: false,
            pending_queued_turn: false,
            turn_handle,
            input_handle,
            _bus_rx: None,
        };
        this.state.set_event_bus(state_bus);
        this
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
    pub async fn run_with_external_rx(mut self, submit_rx: tokio::sync::mpsc::Receiver<Event>) {
        let rx = self
            ._bus_rx
            .take()
            .expect("run_with_external_rx requires UiActor created with with_external_bus_rx");
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
        let (effect_tx, effect_rx) =
            tokio::sync::mpsc::channel::<Event>(EFFECT_FORWARDER_CHANNEL_CAPACITY);
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
    pub(crate) async fn handle_event(
        &mut self,
        evt: Event,
        effect_tx: tokio::sync::mpsc::Sender<Event>,
    ) -> bool {
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
    pub(crate) async fn handle_event_inner(
        &mut self,
        evt: Event,
        effect_tx: tokio::sync::mpsc::Sender<Event>,
    ) -> bool {
        // Priority quit / abort handling.
        //
        // `turn_active` is captured at the very top, BEFORE apply_event runs
        // inside handle_input_event, so the decision reflects the pre-event state.
        let turn_active = self.state.agent_state().turn_active || self.turn_was_active;
        match &evt {
            // Ctrl+Q (ForceQuit) is the "really exit" hatch: always quit, even
            // during an active turn.
            Event::ForceQuit { .. } => {
                // Abort the background file-index scan so the process exits
                // immediately instead of waiting for the initial walk to finish.
                runie_core::actors::fff_indexer::cancel_indexer_scan();
                return true;
            }
            // Ctrl+C (Quit): during a turn, abort the in-flight agent and stay
            // open; when idle, quit (unchanged behavior).
            Event::Quit { .. } => {
                if turn_active {
                    // clear_turn_state(true) cancels the agent's per-turn token
                    // (exactly once) and clears the turn state.
                    self.clear_turn_state(true).await;
                    return false;
                }
                runie_core::actors::fff_indexer::cancel_indexer_scan();
                return true;
            }
            _ => {}
        }

        // Esc / DialogBack at the chat root while a turn is active: abort the
        // turn and stay open. Only fires when no dialog is open, so dialog
        // dismissal is preserved (DialogBack for an open dialog, and vim-nav
        // when idle, flow through apply_event below).
        if matches!(&evt, Event::DialogBack) && self.state.open_dialog().is_none() && turn_active {
            self.clear_turn_state(true).await;
            return false;
        }
        // Capture whether the turn was already active BEFORE apply_event runs.
        // apply_event is called inside handle_input_event, so this must be at the
        // very top to capture the pre-event state.
        let prev_turn_active = self.state.agent_state().turn_active;
        let was_config_loaded = matches!(&evt, Event::ConfigLoaded { .. });

        // Hosted permission-dialog actions: resolve the pending request via the
        // PermissionActor handle and clear the request state.
        self.handle_permission_dialog_action(&evt).await;

        // Track whether `Done` was just applied so `agent_running()` stays true until
        // `TurnCompleted`/`Abort`. Done clears `turn_active` but must not clear the guard.
        self.handle_input_event(&evt).await;

        if !matches!(&evt, Event::InputChanged { .. }) {
            self.update_paced_renderer(&evt);
            effects::dispatch(self, &evt, effect_tx.clone()).await;
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
        if matches!(
            &evt,
            Event::FollowUpDelivered { .. } | Event::SteeringDelivered { .. }
        ) {
            self.pending_queued_turn = true;
        }

        if let Event::TurnStarted {
            request_id,
            content,
            ..
        } = &evt
        {
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
        if matches!(
            &evt,
            Event::TurnCompleted | Event::TurnErrored { .. } | Event::Abort
        ) {
            let is_abort = matches!(&evt, Event::Abort);
            self.clear_turn_state(is_abort).await;
        }

        false
    }

    /// Handle hosted permission-dialog actions emitted by the dialog panel.
    ///
    /// Resolves the pending request through the PermissionActor handle and clears
    /// the request state so the UI and the waiting agent move forward together.
    async fn handle_permission_dialog_action(&mut self, evt: &Event) {
        let request_id = match evt {
            Event::PermissionAllow { request_id } => request_id.clone(),
            Event::PermissionDeny { request_id } => request_id.clone(),
            Event::PermissionAlwaysAllow { request_id, .. } => request_id.clone(),
            Event::PermissionSessionAllow { request_id, .. } => request_id.clone(),
            Event::PermissionOnce { request_id } => request_id.clone(),
            _ => return,
        };

        let Some(req) = self.state.permission_request_opt() else {
            return;
        };
        if req.request_id != request_id {
            return;
        }

        let action = match evt {
            Event::PermissionAllow { .. } => PermissionAction::Allow,
            Event::PermissionDeny { .. } => PermissionAction::Deny,
            Event::PermissionAlwaysAllow { tool, .. } => {
                if let Some(handles) = self.state.actor_handles() {
                    handles
                        .permission
                        .try_upsert_rule(tool.clone(), PermissionAction::Allow);
                }
                PermissionAction::Allow
            }
            Event::PermissionSessionAllow { tool, .. } => {
                if let Some(handles) = self.state.actor_handles() {
                    handles
                        .permission
                        .try_upsert_session_rule(tool.clone(), PermissionAction::Allow);
                }
                PermissionAction::Allow
            }
            Event::PermissionOnce { .. } => {
                // Once: just allow this single request, no rule persistence
                PermissionAction::Allow
            }
            _ => return,
        };

        if let Some(handles) = self.state.actor_handles() {
            handles
                .permission
                .try_resolve_permission(request_id.clone(), action);
        }

        let dismiss = Event::PermissionRequestDismissed;
        self.bus.publish(dismiss.clone());
        self.apply_event(dismiss);
    }

    /// Route input events through InputActor instead of applying directly.
    /// Route input events through `route_to_input_actor` (the canonical mapping).
    /// UiActor-specific cases (Submit, InputChanged) are handled separately;
    /// everything else is routed via the shared helper.
    ///
    /// UiActor must NEVER mutate `AppState.input` directly — only through `apply_event`.
    async fn handle_input_event(&mut self, evt: &Event) {
        // Synchronous autocomplete trigger: open the command palette/file picker
        // immediately when '/' or '@' is typed at a trigger position. This prevents
        // a race where the dialog opens asynchronously after subsequent key events
        // have already been routed to the chat input, leaving the palette filter
        // empty and causing Enter to run the first item (/approve).
        //
        // The AppState input projection lags the InputActor by one InputChanged
        // round-trip, so the trigger check must also consider characters we
        // have already routed but not yet seen echoed back
        // (`pending_input_chars`); otherwise '/' typed right after text (e.g.
        // a path like `src/main.rs`) sees a stale-empty input and opens the
        // palette, swallowing the text.
        if let Event::Input(c) = evt {
            if self.state.open_dialog().is_none() && !self.state.view().vim_nav_mode {
                if self.open_autocomplete_if_trigger(*c).await {
                    return;
                }
                // No dialog and no vim nav: this character will be routed to
                // the InputActor below. Mirror it optimistically so the next
                // keystroke's trigger check sees it.
                self.pending_input_chars.push(*c);
            }
        }

        // Dialog input guard: when a dialog is open, apply typing/navigation/submit
        // events directly to state so the dialog form/palette receives them. The
        // canonical router would otherwise send these to InputActor, which only
        // mutates the chat input box and ignores modal forms (e.g. onboarding login flow).
        // This also covers the hosted permission panel, which is a Generic dialog.
        if self.state.open_dialog().is_some() && helpers::is_dialog_input_event(evt) {
            self.apply_event(evt.clone());
            return;
        }

        // Vim nav mode intercepts keys that would otherwise edit the chat input.
        // Route them through the canonical state update so j/k/i/I/space/arrows
        // move the feed selection or return to the input box. Enter (Submit) is
        // included: in nav mode it expands/collapses the selected post (or keeps
        // its legacy global-toggle fallback) — it must NOT submit the chat input.
        if self.state.view().vim_nav_mode {
            match evt {
                Event::Input(_)
                | Event::Submit
                | Event::HistoryPrev
                | Event::HistoryNext
                | Event::Backspace => {
                    self.apply_event(evt.clone());
                    return;
                }
                _ => {}
            }
        }

        // Empty-input ↑/↓ scroll the feed instead of cycling prompt history.
        // The canonical router would send these straight to the InputActor,
        // bypassing the core history-nav mode dispatch (HistoryNavMode::Scroll).
        // Terminals with "alternate scroll" (iTerm2, kitty, WezTerm) translate
        // mouse-wheel ticks into arrow keys when the app does not capture the
        // mouse (runie keeps native selection), so wheel events arrive as
        // HistoryPrev/HistoryNext and must scroll, not recall history.
        // `effective_input_content` includes the optimistic pending mirror so
        // fast typing (echo not yet processed) still counts as non-empty.
        if matches!(evt, Event::HistoryPrev | Event::HistoryNext)
            && self.effective_input_content().is_empty()
        {
            self.apply_event(evt.clone());
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
            Event::Input(_c) => {
                // Non-permission Input events would have been routed above.
                // Permission decisions are now handled through the hosted dialog
                // panel and the PermissionAllow/Deny/AlwaysAllow events.
            }
            Event::Submit => {
                // Quit commands must exit immediately, without waiting for the
                // InputActor round-trip that normal submit flow requires.
                let content = self.effective_input_content();
                if runie_core::update::input::is_quit_command(content.trim()) {
                    // Abort the background file-index scan so the process exits
                    // immediately instead of waiting for the initial walk.
                    runie_core::actors::fff_indexer::cancel_indexer_scan();
                    *self.state.should_quit_mut() = true;
                    return;
                }
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

    /// Handle the Submit event when no modal dialog is open.
    ///
    /// Dialog forms and palettes receive Enter via `is_dialog_input_event`, so
    /// this path only submits the chat input box.
    async fn handle_submit_event(&mut self) {
        let content = self.effective_input_content().trim().to_owned();
        self.pending_submit = if content.is_empty() {
            None
        } else {
            Some(content.clone())
        };
        // The submit flow clears the input box; the optimistic mirror resets too.
        self.pending_input_chars.clear();
        self.send_input_msg(runie_core::actors::InputMsg::Submit { content })
            .await;
    }

    /// The full chat input content: the AppState projection plus characters
    /// routed to the InputActor whose `InputChanged` echo has not been
    /// processed yet. The projection alone lags real typing by one
    /// round-trip, so submit/quit checks must include the pending mirror or
    /// fast typing loses its trailing characters.
    fn effective_input_content(&self) -> String {
        let pending: String = self.pending_input_chars.iter().collect();
        format!("{}{}", self.state.input().input(), pending)
    }

    /// Handle InputChanged: route through apply_event so all state mutations
    /// flow through one canonical path, then trigger side effects.
    /// UiActor must NEVER mutate AppState.input directly — only through apply_event.
    async fn handle_input_changed(&mut self, state: &runie_core::InputState) {
        // Capture prev_input BEFORE apply_event changes self.state.input.
        // The projection still holds the pre-change content at this point;
        // reading it here keeps the autocomplete trigger in sync with what
        // the user actually typed (a cached field would go stale).
        let prev_input = self.state.input().input.clone();
        let prev_cursor_pos = self.state.input().cursor_pos;
        let new_input = state.input().to_owned();
        let new_cursor_pos = state.cursor_pos;

        // Each routed character produces exactly one InputChanged echo; drop
        // it from the optimistic pending mirror. Clears/pastes leave the
        // queue untouched because those paths reset it themselves.
        if !self.pending_input_chars.is_empty() {
            self.pending_input_chars.remove(0);
        }

        // Route through apply_event — the single source of truth for state mutations.
        // UiActor must NOT mutate AppState.input directly.
        self.apply_event(Event::InputChanged {
            state: Box::new((*state).clone()),
        });

        self.detect_autocomplete_trigger(&prev_input, prev_cursor_pos, &new_input, new_cursor_pos)
            .await;

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
                // Reset the paced renderer so it doesn't show stale streaming_tail
                // after the response has been committed to the feed as AgentMessage.
                self.paced = PacedRenderer::new();
            }
            _ => {}
        }
    }

    fn apply_event(&mut self, evt: Event) {
        self.state.update(evt);
    }

    /// Build a snapshot with the paced streaming tail applied.
    fn build_paced_snapshot(&mut self) -> Snapshot {
        self.state.ensure_fresh();
        let mut snap = self.state.snapshot();
        // Only show streaming tail when turn is active.
        // When turn_active is false, the pacing renderer may contain stale content
        // from the previous turn, so we clear it to avoid showing old responses.
        if snap.turn_active {
            snap.streaming_tail = self.paced.displayed().to_owned();
        } else {
            snap.streaming_tail = String::new();
        }

        snap
    }

    /// Fire-and-forget send to InputActor.
    async fn send_input_msg(&self, msg: runie_core::actors::InputMsg) {
        if let Some(ref handle) = self.input_handle {
            let _ = handle.send_message(msg);
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
        // Force turn_active=true for the final snapshot so streaming_tail is rendered.
        // TurnActor already cleared it, but we need it true here to capture the complete
        // streamed response text in the snapshot before it gets cleared.
        self.state.agent_state_mut().turn_active = true;
        let snap = self.build_paced_snapshot();
        let _ = self.render_tx.send(snap);
        self.state.agent_state_mut().turn_active = false;
        self.turn_was_active = false;
        self.pending_queued_turn = false;
        if is_abort {
            // Cancel the in-flight agent (per-turn CancellationToken) so Ctrl+C,
            // Esc, Ctrl+S and /new actually stop the stream — not just the UI.
            // Safe even when idle: token.cancel is idempotent and the handle
            // abort is a harmless no-op when nothing is running.
            self.agent_handle.abort().await;
        }
        if let Some(ref turn_handle) = self.turn_handle {
            if is_abort {
                // Abort: clear the queue so a new session starts clean.
                turn_handle
                    .send(runie_core::actors::TurnMsg::ClearQueues)
                    .await;
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
        submit::dispatch(self, content).await;
    }

    /// If a form panel is open, emit CommandFormSubmit and return true.
    /// Returns `false` if no form panel is open, so the caller knows to use the
    /// fallback behavior (close dialog and handle as slash command).
    pub(crate) fn maybe_submit_form(&mut self) -> bool {
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
