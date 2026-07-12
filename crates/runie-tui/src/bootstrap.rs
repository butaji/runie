//! Testable TUI bootstrap with configurable backend and input.
//!
//! This module provides a `TuiRuntime` struct that encapsulates all TUI runtime
//! components and can be configured with custom backends, providers, and input sources
//! for deterministic testing.
//!
//! ## Usage
//!
//! ```ignore
//! use runie_tui::bootstrap::{TuiRuntime, Keystroke, BackendType};
//! use ratatui::backend::TestBackend;
//! use runie_provider::BuiltProviderFactory;
//!
//! let runtime = TuiRuntime::builder()
//!     .provider_factory(Arc::new(BuiltProviderFactory::new()))
//!     .backend(BackendType::Test(TestBackend::new(80, 24)))
//!     .keystrokes(vec![Keystroke::Char('H'), Keystroke::Char('i'), Keystroke::Enter])
//!     .build()?;
//!
//! runtime.run().await;
//! ```

use std::{collections::HashMap, sync::Arc};

use futures::StreamExt;
use ratatui::backend::TestBackend;
use runie_agent::AgentActorFactoryImpl;
use runie_core::actors::leader::{Leader, LeaderHandle};
use runie_core::actors::RactorTurnHandle;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::{AppState, Snapshot};
use runie_provider::BuiltProviderFactory;
use tokio::sync::{mpsc, oneshot, watch};

use crate::{
    app_init, input_mapping, keymap, terminal, terminal_setup, theme, ui,
    ui_actor::{AgentHandleBox, LeaderAgentActorHandle, UiActor},
};

/// Backend type for TUI rendering.
#[derive(Clone)]
pub enum BackendType {
    /// Real Crossterm backend for production.
    Crossterm,
    /// Test backend for deterministic rendering tests.
    Test(TestBackend),
}

/// Holds all spawned task handles for the TUI runtime.
///
/// Every spawned task is tracked here and awaited on shutdown.
/// This ensures no orphan tasks and panics are observable.
#[derive(Default)]
pub struct TuiRuntimeHandles {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl TuiRuntimeHandles {
    /// Spawn a new task and track its handle.
    pub fn spawn(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.handles.push(handle);
    }

    /// Abort and await all spawned tasks with a short timeout.
    ///
    /// Background tasks such as the crossterm input reader are blocked on I/O
    /// and will not observe the shutdown signal until a new event arrives, so
    /// we abort them explicitly to ensure the process exits immediately when
    /// the user quits.
    pub async fn shutdown(mut self) {
        for handle in &self.handles {
            handle.abort();
        }
        let timeout = std::time::Duration::from_millis(500);
        let _ = tokio::time::timeout(timeout, async {
            while let Some(handle) = self.handles.pop() {
                if let Err(e) = handle.await {
                    tracing::debug!(?e, "TUI runtime task exited with error");
                }
            }
        })
        .await;
    }
}

/// Keystroke DSL for programmatic input simulation.
#[derive(Debug, Clone)]
pub enum Keystroke {
    /// Character input.
    Char(char),
    /// Enter/Submit.
    Enter,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Arrow keys.
    Up,
    Down,
    Left,
    Right,
    /// Ctrl modifier combination.
    Ctrl(char),
    /// Alt modifier combination.
    Alt(char),
    /// Tab.
    Tab,
    /// Ctrl+C (quit).
    CtrlC,
    /// Ctrl+O (toggle expand).
    CtrlO,
    /// Ctrl+L (clear).
    CtrlL,
    /// Ctrl+U (clear line).
    CtrlU,
    /// Ctrl+A (cursor start).
    CtrlA,
    /// Ctrl+E (cursor end).
    CtrlE,
    /// Ctrl+K (kill after cursor).
    CtrlK,
    /// Ctrl+W (kill word).
    CtrlW,
    /// Ctrl+B (vim left).
    CtrlB,
    /// Ctrl+F (vim right / forward char).
    CtrlF,
    /// Ctrl+P (history prev).
    CtrlP,
    /// Ctrl+N (history next).
    CtrlN,
    /// Alt+Enter (follow up).
    AltEnter,
    /// Ctrl+\ (abort).
    CtrlBackslash,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Delete key.
    Delete,
    /// Raw runie_core event for advanced use (bypasses keymap conversion).
    RawEvent(Event),
}

impl Keystroke {
    /// Convert to a runie_core Event using the keymap.
    ///
    /// Returns `None` if the keystroke should be ignored (e.g., Ctrl+Shift+E).
    /// For `RawEvent`, returns the event directly without keymap conversion.
    pub fn to_event(&self, user_bindings: &HashMap<String, String>) -> Option<Event> {
        // RawEvent bypasses keymap conversion and returns directly
        if let Keystroke::RawEvent(event) = self {
            return Some(event.clone());
        }
        let crossterm_event = self.to_crossterm_event();
        keymap::convert_event(&crossterm_event, user_bindings)
    }

    /// Convert to a crossterm event (for advanced use).
    fn to_crossterm_event(&self) -> crossterm::event::Event {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        match self {
            Keystroke::Char(c) => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char(*c),
                KeyModifiers::empty(),
            )),
            Keystroke::Enter => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))
            }
            Keystroke::Backspace => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Backspace,
                KeyModifiers::empty(),
            )),
            Keystroke::Escape => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            }
            Keystroke::Up => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()))
            }
            Keystroke::Down => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()))
            }
            Keystroke::Left => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()))
            }
            Keystroke::Right => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()))
            }
            Keystroke::Ctrl(c) => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char(*c),
                KeyModifiers::CONTROL,
            )),
            Keystroke::Alt(c) => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char(*c), KeyModifiers::ALT))
            }
            Keystroke::Tab => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()))
            }
            Keystroke::CtrlC => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlO => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('o'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlL => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('l'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlU => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlA => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlE => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlK => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlW => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('w'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlB => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('b'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlF => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlP => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('p'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::CtrlN => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::AltEnter => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT))
            }
            Keystroke::CtrlBackslash => crossterm::event::Event::Key(KeyEvent::new(
                KeyCode::Char('\\'),
                KeyModifiers::CONTROL,
            )),
            Keystroke::Home => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()))
            }
            Keystroke::End => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()))
            }
            Keystroke::Delete => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty()))
            }
            // RawEvent is handled in to_event() directly
            Keystroke::RawEvent(_) => {
                crossterm::event::Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            }
        }
    }
}

/// Builder for `TuiRuntime`.
pub struct TuiRuntimeBuilder {
    provider_factory: Arc<dyn runie_core::actors::provider::ProviderFactory>,
    backend: BackendType,
    keystrokes: Vec<Keystroke>,
    project_root: std::path::PathBuf,
    data_dir: std::path::PathBuf,
}

impl Default for TuiRuntimeBuilder {
    fn default() -> Self {
        Self {
            provider_factory: Arc::new(BuiltProviderFactory::new()),
            backend: BackendType::Crossterm,
            keystrokes: Vec::new(),
            project_root: std::env::current_dir().unwrap_or_default(),
            data_dir: dirs::data_dir().unwrap_or_else(std::env::temp_dir),
        }
    }
}

impl TuiRuntimeBuilder {
    /// Set the provider factory.
    pub fn provider_factory(
        mut self,
        factory: Arc<dyn runie_core::actors::provider::ProviderFactory>,
    ) -> Self {
        self.provider_factory = factory;
        self
    }

    /// Set the backend type.
    pub fn backend(mut self, backend: BackendType) -> Self {
        self.backend = backend;
        self
    }

    /// Set the keystroke sequence.
    pub fn keystrokes(mut self, keystrokes: Vec<Keystroke>) -> Self {
        self.keystrokes = keystrokes;
        self
    }

    /// Set the project root directory.
    pub fn project_root(mut self, path: std::path::PathBuf) -> Self {
        self.project_root = path;
        self
    }

    /// Set the data directory.
    pub fn data_dir(mut self, path: std::path::PathBuf) -> Self {
        self.data_dir = path;
        self
    }

    /// Build the `TuiRuntime`.
    pub fn build(self) -> TuiRuntime {
        TuiRuntime {
            provider_factory: self.provider_factory,
            backend: self.backend,
            keystrokes: self.keystrokes,
            project_root: self.project_root,
            data_dir: self.data_dir,
        }
    }
}

/// Runtime container for the TUI application.
///
/// This struct holds all components needed to run the TUI and provides
/// a unified interface for both production and testing use cases.
#[derive(Clone)]
#[allow(dead_code)]
pub struct TuiRuntime {
    provider_factory: Arc<dyn runie_core::actors::provider::ProviderFactory>,
    backend: BackendType,
    keystrokes: Vec<Keystroke>,
    project_root: std::path::PathBuf,
    data_dir: std::path::PathBuf,
}

impl Default for TuiRuntime {
    fn default() -> Self {
        TuiRuntimeBuilder::default().build()
    }
}

impl TuiRuntime {
    /// Create a new runtime builder.
    pub fn builder() -> TuiRuntimeBuilder {
        TuiRuntimeBuilder::default()
    }

    /// Get the backend type.
    pub fn backend(&self) -> &BackendType {
        &self.backend
    }

    /// Get the keystroke sequence.
    pub fn keystrokes(&self) -> &[Keystroke] {
        &self.keystrokes
    }

    /// Run the TUI runtime.
    ///
    /// In production mode, this reads from the terminal and blocks until quit.
    /// In test mode with keystrokes, this runs the keystroke sequence and returns.
    pub async fn run(&self) -> std::io::Result<()> {
        // Create the event bus upfront so that UiActor can subscribe before actors emit
        // initial facts (ConfigLoaded, TrustLoaded, HistoryLoaded).
        let bus = EventBus::<Event>::new(1000);

        // Subscribe UiActor to the bus BEFORE starting the leader so it receives
        // ConfigLoaded and other initial facts that are emitted during actor spawn.
        let bus_rx = bus.subscribe();

        let leader = Leader::new();
        let agent_factory = std::sync::Arc::new(AgentActorFactoryImpl);
        let leader_handle = match leader
            .start_with_bus(self.provider_factory.clone(), agent_factory, bus.clone())
            .await
        {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Leader bootstrap failed: {:#}", e);
                tracing::error!("Hint: Set RUST_LOG=debug for more details.");
                return Ok(());
            }
        };

        let mut state = AppState::default();
        state.set_actor_handles(leader_handle.clone());
        app_init::bootstrap(&mut state).await;

        // Clone state for test mode (we need it for keybindings)
        let state_for_keys = state.clone();

        match &self.backend {
            BackendType::Crossterm => self.run_production(state, bus_rx, leader_handle).await,
            BackendType::Test(_) => {
                // For test mode, we run with the provided keystrokes and then quit
                self.run_with_keystrokes(state, state_for_keys, bus_rx, leader_handle)
                    .await
            }
        }
    }

    /// Production run with real terminal.
    async fn run_production(
        &self,
        mut state: AppState,
        bus_rx: runie_core::bus::Receiver<Event>,
        leader_handle: LeaderHandle,
    ) -> std::io::Result<()> {
        let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
        init_terminal_state(&mut state);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut handles = TuiRuntimeHandles::default();
        spawn_background_tasks(
            terminal,
            state,
            terminal_caps,
            leader_handle.clone(),
            bus_rx,
            shutdown_tx,
            &mut handles,
        )
        .await;

        // Wait for the in-app quit signal OR an OS SIGTERM/SIGINT, so a plain
        // `kill` does not strand the terminal in raw mode / alternate screen.
        wait_for_shutdown(shutdown_rx).await;

        // Graceful teardown on every exit path. `leader_handle.shutdown()` is
        // idempotent (never panics; safe even if a clone already shut the leader
        // down via the in-app quit key) and `handles.shutdown()` awaits the rest.
        leader_handle.shutdown().await;
        handles.shutdown().await;
        Ok(())
    }

    /// Test run with keystroke sequence.
    async fn run_with_keystrokes(
        &self,
        mut state: AppState,
        state_for_keys: AppState,
        bus_rx: runie_core::bus::Receiver<Event>,
        leader_handle: LeaderHandle,
    ) -> std::io::Result<()> {
        let backend = match &self.backend {
            BackendType::Test(b) => b.clone(),
            BackendType::Crossterm => {
                return Err(std::io::Error::other(
                    "Cannot use keystrokes with Crossterm backend",
                ));
            }
        };

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut handles = TuiRuntimeHandles::default();
        let (terminal, terminal_caps) = setup_test_terminal(backend);
        init_terminal_state(&mut state);

        // Create channels for event routing
        let (submit_tx, submit_rx) = mpsc::channel::<Event>(16);

        // Spawn UI actor
        let bus = leader_handle.event_bus().clone();
        let (kb_tx, _kb_rx) = watch::channel(state.config().keybindings().clone());
        let mut ui_actor = spawn_ui_actor_with_external_rx(
            state,
            bus_rx,
            leader_handle.turn.clone(),
            leader_handle.input.clone(),
            kb_tx,
            bus.clone(),
            shutdown_tx,
            terminal_caps,
        );
        ui_actor.set_agent_handle(AgentHandleBox::Leader(LeaderAgentActorHandle::new(
            leader_handle.agent.clone(),
        )));
        let render_rx = ui_actor.take_render_rx();
        handles.spawn(tokio::spawn(async move {
            ui_actor.run_with_external_rx(submit_rx).await;
        }));

        // Spawn test render loop
        let throbber = std::sync::Arc::new(parking_lot::Mutex::new(
            throbber_widgets_tui::ThrobberState::default(),
        ));
        handles.spawn(tokio::spawn(test_render_loop(
            terminal, render_rx, throbber,
        )));

        // Feed keystrokes - convert to runie_core events and send to input forwarder
        let user_bindings = state_for_keys.config().keybindings().clone();
        for ks in &self.keystrokes {
            let evt = match ks {
                Keystroke::RawEvent(event) => Some(event.clone()),
                _ => ks.to_event(&user_bindings),
            };

            if let Some(evt) = evt {
                let is_quit = is_input_stop_event(&evt);
                // Route to InputActor via the canonical router
                if input_mapping::route_to_input_actor(&leader_handle.input, &evt).await {
                    continue;
                }
                // Non-routable events go directly to UiActor
                let _ = submit_tx.send(evt).await;
                if is_quit {
                    break;
                }
            }
        }

        // Yield to let rendering settle
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Signal shutdown
        let _ = shutdown_rx.await;

        // Await all spawned tasks to ensure clean shutdown.
        handles.shutdown().await;
        Ok(())
    }
}

/// Initialize terminal state from actual terminal size.
fn init_terminal_state(state: &mut AppState) {
    if let Ok((width, height)) = crossterm::terminal::size() {
        // Use area width (terminal width minus left/right margins) so the value
        // matches what render uses: `content_width = area.width.saturating_sub(2)`.
        // Previously we passed the raw terminal width, causing a double-subtract
        // that made content_width = 76 instead of 78 in an 80-wide terminal.
        let area_width = width.saturating_sub(2);
        state.set_last_content_width(area_width);
        state.set_last_visible_height(height);
    }
}

/// Spawn background tasks for the TUI.
async fn spawn_background_tasks(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    state: AppState,
    caps: terminal::caps::TermCaps,
    leader_handle: LeaderHandle,
    bus_rx: runie_core::bus::Receiver<Event>,
    shutdown_tx: oneshot::Sender<()>,
    handles: &mut TuiRuntimeHandles,
) {
    let bus = leader_handle.event_bus().clone();
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (submit_tx, submit_rx) = mpsc::channel::<Event>(16);

    let (kb_tx, kb_rx) = watch::channel(state.config().keybindings().clone());
    handles.spawn(tokio::spawn(input_forwarder_task(input_rx, submit_tx)));

    // UiActor was created before start_with_bus() with a NoOp agent handle and
    // the pre-subscribed bus_rx. Install the real agent handle and run it.
    let mut ui_actor = spawn_ui_actor_with_external_rx(
        state,
        bus_rx,
        leader_handle.turn.clone(),
        leader_handle.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui_actor.set_agent_handle(AgentHandleBox::Leader(LeaderAgentActorHandle::new(
        leader_handle.agent.clone(),
    )));
    let render_rx = ui_actor.take_render_rx();
    handles.spawn(tokio::spawn(async move {
        ui_actor.run_with_external_rx(submit_rx).await;
    }));

    spawn_agent_tasks(input_tx, kb_rx, terminal, render_rx, caps, handles);
}

fn spawn_agent_tasks(
    input_tx: mpsc::Sender<Event>,
    kb_rx: watch::Receiver<HashMap<String, String>>,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    render_rx: watch::Receiver<Snapshot>,
    caps: terminal::caps::TermCaps,
    handles: &mut TuiRuntimeHandles,
) {
    handles.spawn(tokio::spawn(input_reader(input_tx, kb_rx)));
    handles.spawn(tokio::spawn(async_render_loop(terminal, render_rx, caps)));
}

/// Forwarder: bridge raw terminal events to the UiActor submit channel.
///
/// All terminal input is routed through UiActor so it can decide whether the
/// event belongs to a modal dialog/form (onboarding, palettes, settings) or to
/// the main chat input box. Sending everything to InputActor directly bypassed
/// dialog forms and broke onboarding typing/arrows.
async fn input_forwarder_task(mut input_rx: mpsc::Receiver<Event>, submit_tx: mpsc::Sender<Event>) {
    while let Some(evt) = input_rx.recv().await {
        let _ = submit_tx.send(evt).await;
    }
}

/// Wrapper for Terminal that can be shared across blocking tasks.
struct RenderTerminal {
    inner: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
}

impl RenderTerminal {
    fn size(&self) -> std::io::Result<(u16, u16)> {
        self.inner.size().map(|r| (r.width, r.height))
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.inner.clear().map(|_| ())
    }

    fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> std::io::Result<()> {
        self.inner.draw(f).map(|_| ())
    }
}

/// Async render loop using tokio watch channel.
async fn async_render_loop(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut render_rx: watch::Receiver<Snapshot>,
    caps: terminal::caps::TermCaps,
) {
    let mut last_size: Option<(u16, u16)> = None;
    let throbber = std::sync::Arc::new(parking_lot::Mutex::new(
        throbber_widgets_tui::ThrobberState::default(),
    ));
    let term = std::sync::Arc::new(parking_lot::Mutex::new(RenderTerminal { inner: terminal }));

    loop {
        if render_rx.changed().await.is_err() {
            break;
        }

        let snap = render_rx.borrow().clone();

        let term_clone = Arc::clone(&term);
        let new_size = match tokio::task::spawn_blocking(move || {
            let guard = term_clone.lock();
            guard.size()
        })
        .await
        {
            Ok(Ok(s)) => s,
            _ => continue,
        };

        if last_size != Some(new_size) {
            let term_clone = Arc::clone(&term);
            let _ = tokio::task::spawn_blocking(move || {
                let mut guard = term_clone.lock();
                guard.clear()
            })
            .await;
            last_size = Some(new_size);
        }

        theme::set_current_theme_with_caps(&snap.theme_name, caps);

        let term_clone = Arc::clone(&term);
        let throbber_clone = Arc::clone(&throbber);
        let snap = snap;
        let _ = tokio::task::spawn_blocking(move || {
            let mut term_guard = term_clone.lock();
            let mut throbber_guard = throbber_clone.lock();
            term_guard.draw(|f| ui::draw_snapshot(f, &snap, &mut throbber_guard))
        })
        .await;
    }
}

/// Test render loop for deterministic testing.
async fn test_render_loop(
    mut terminal: ratatui::Terminal<TestBackend>,
    mut render_rx: watch::Receiver<Snapshot>,
    throbber: Arc<parking_lot::Mutex<throbber_widgets_tui::ThrobberState>>,
) {
    while let Ok(()) = render_rx.changed().await {
        let snap = render_rx.borrow().clone();
        let _ = terminal.draw(|f| {
            ui::draw_snapshot(f, &snap, &mut throbber.lock());
        });
    }
}

/// Setup a test terminal with the given backend.
fn setup_test_terminal(
    backend: TestBackend,
) -> (ratatui::Terminal<TestBackend>, terminal::caps::TermCaps) {
    let terminal = ratatui::Terminal::new(backend).expect("test terminal");
    let caps = terminal::caps::TermCaps::default();
    (terminal, caps)
}

/// Create a UiActor with a pre-subscribed bus receiver.
/// Use this when the bus receiver was created before `Leader::start_with_bus()` returns.
#[allow(clippy::too_many_arguments)]
fn spawn_ui_actor_with_external_rx(
    state: AppState,
    bus_rx: runie_core::bus::Receiver<Event>,
    turn_handle: RactorTurnHandle,
    input_handle: runie_core::actors::RactorInputHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
    caps: terminal::caps::TermCaps,
) -> UiActor {
    UiActor::with_external_bus_rx(
        state,
        bus_rx,
        turn_handle,
        input_handle,
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    )
}

async fn input_reader(
    input_tx: mpsc::Sender<Event>,
    mut kb_rx: watch::Receiver<HashMap<String, String>>,
) {
    let mut reader = crossterm::event::EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        let bindings = kb_rx.borrow_and_update().clone();
        if let Some(evt) = keymap::convert_event(&event, &bindings) {
            let is_quit = is_input_stop_event(&evt);
            if input_tx.send(evt).await.is_err() {
                break;
            }
            if is_quit {
                break;
            }
        }
    }
}

/// Returns true for events that should stop the input reader after dispatching.
/// ForceQuit must be included so the input task exits immediately and the app
/// shuts down without waiting for the shutdown timeout.
fn is_input_stop_event(evt: &Event) -> bool {
    matches!(evt, Event::Quit | Event::Reset | Event::ForceQuit)
}

/// Wait for either the in-app shutdown signal or an OS termination signal
/// (SIGTERM/SIGINT on Unix), returning as soon as any of them fire.
///
/// A plain `kill` (SIGTERM) — or a Ctrl+C delivered outside the raw-mode TTY —
/// would otherwise terminate the process before the `Cleanup` drop guard can
/// restore the terminal, stranding the user's shell in alternate screen / raw
/// mode. Catching the signal and returning normally lets `run_production` run
/// its graceful teardown, after which the guard restores the terminal on exit.
async fn wait_for_shutdown(shutdown_rx: oneshot::Receiver<()>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).ok();
        let mut sigint = signal(SignalKind::interrupt()).ok();
        tokio::pin!(shutdown_rx);
        tokio::select! {
            _ = &mut shutdown_rx => {}
            _ = recv_optional_signal(&mut sigterm) => {
                tracing::warn!("SIGTERM received; shutting down gracefully");
            }
            _ = recv_optional_signal(&mut sigint) => {
                tracing::warn!("SIGINT received; shutting down gracefully");
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = shutdown_rx.await;
    }
}

/// Resolve when an optional Unix signal stream fires, or never if it is `None`
/// (signal registration failed). Drives one arm of `wait_for_shutdown`'s select.
#[cfg(unix)]
async fn recv_optional_signal(sig: &mut Option<tokio::signal::unix::Signal>) {
    match sig {
        Some(s) => {
            s.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_to_event_char() {
        let ks = Keystroke::Char('x');
        let bindings = HashMap::new();
        let event = ks.to_event(&bindings);
        assert!(matches!(event, Some(Event::Input('x'))));
    }

    #[test]
    fn keystroke_to_event_ctrl_c() {
        let ks = Keystroke::CtrlC;
        let bindings = HashMap::new();
        let event = ks.to_event(&bindings);
        assert!(matches!(event, Some(Event::Quit)));
    }

    #[test]
    fn keystroke_to_event_escape() {
        let ks = Keystroke::Escape;
        let bindings = HashMap::new();
        let event = ks.to_event(&bindings);
        assert!(matches!(event, Some(Event::DialogBack)));
    }

    #[test]
    fn keystroke_raw_event() {
        let ks = Keystroke::RawEvent(Event::Quit);
        let bindings = HashMap::new();
        let event = ks.to_event(&bindings);
        // RawEvent should return the raw event directly
        assert!(matches!(event, Some(Event::Quit)));
    }

    #[test]
    fn keystroke_alt_enter() {
        let ks = Keystroke::AltEnter;
        let bindings = HashMap::new();
        let event = ks.to_event(&bindings);
        assert!(matches!(event, Some(Event::FollowUp)));
    }

    #[test]
    fn runtime_builder_default() {
        let runtime = TuiRuntime::builder().build();
        assert!(matches!(runtime.backend, BackendType::Crossterm));
        assert!(runtime.keystrokes.is_empty());
    }

    #[test]
    fn runtime_builder_with_test_backend() {
        let backend = TestBackend::new(80, 24);
        let runtime = TuiRuntime::builder()
            .backend(BackendType::Test(backend))
            .keystrokes(vec![Keystroke::Char('H'), Keystroke::Char('i')])
            .build();
        assert!(matches!(runtime.backend, BackendType::Test(_)));
        assert_eq!(runtime.keystrokes.len(), 2);
    }

    #[test]
    fn keystroke_sequence() {
        let bindings = HashMap::new();
        let keystrokes = [Keystroke::Char('H'), Keystroke::Char('i'), Keystroke::Enter];
        let events: Vec<_> = keystrokes
            .iter()
            .filter_map(|ks| ks.to_event(&bindings))
            .collect();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], Event::Input('H')));
        assert!(matches!(events[1], Event::Input('i')));
        // Enter maps to Submit via the keymap
        assert!(matches!(events[2], Event::Submit));
    }

    #[test]
    fn input_stop_event_includes_quit() {
        assert!(is_input_stop_event(&Event::Quit));
    }

    #[test]
    fn input_stop_event_includes_reset() {
        assert!(is_input_stop_event(&Event::Reset));
    }

    #[test]
    fn input_stop_event_includes_force_quit() {
        assert!(
            is_input_stop_event(&Event::ForceQuit),
            "ForceQuit must stop the input reader so shutdown is immediate"
        );
    }

    #[test]
    fn input_stop_event_rejects_other_events() {
        assert!(!is_input_stop_event(&Event::Submit));
        assert!(!is_input_stop_event(&Event::Input('q')));
        assert!(!is_input_stop_event(&Event::DialogBack));
    }

    /// Layer 1: TuiRuntimeHandles stores all spawned task handles.
    #[tokio::test]
    async fn runtime_handles_stores_task_handles() {
        let mut handles = TuiRuntimeHandles::default();
        assert!(handles.handles.is_empty());

        // Spawn a simple task and capture its handle.
        let handle = tokio::spawn(async {});
        handles.spawn(handle);
        assert_eq!(handles.handles.len(), 1);
    }

    /// Layer 1: TuiRuntimeHandles::shutdown awaits all spawned tasks.
    #[tokio::test]
    async fn runtime_handles_shutdown_awaits_tasks() {
        let mut handles = TuiRuntimeHandles::default();

        // Spawn multiple tasks.
        handles.spawn(tokio::spawn(async {}));
        handles.spawn(tokio::spawn(async {}));
        handles.spawn(tokio::spawn(async {}));

        assert_eq!(handles.handles.len(), 3);

        // Shutdown takes ownership and awaits all tasks without panicking.
        handles.shutdown().await;
        // After shutdown, handles is consumed.
    }

    /// Layer 1: `wait_for_shutdown` resolves promptly when the in-app oneshot
    /// fires (the common quit path), so wiring it into `run_production` does not
    /// regress normal shutdown. The OS-signal arms are exercised live; they are
    /// not unit-tested here because delivering SIGTERM to the test process would
    /// terminate it.
    #[tokio::test]
    async fn wait_for_shutdown_resolves_on_oneshot() {
        let (tx, rx) = oneshot::channel();
        let waiter = tokio::spawn(wait_for_shutdown(rx));
        tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), waiter)
            .await
            .expect("wait_for_shutdown must resolve when the shutdown oneshot fires")
            .unwrap();
    }
}
