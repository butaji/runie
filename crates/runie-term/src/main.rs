//! Runie Terminal — Non-blocking event loop with render actor
//!
//! Architecture (impossible to block by design):
//!   1. Event loop: single-threaded async, only async ops
//!   2. State: owned by event loop, mutable borrow per event
//!   3. Snapshot: immutable frame description (the UI DSL)
//!   4. Render actor: owns Terminal, receives Snapshots via channel
//!   5. If render is slow, old Snapshots are dropped — event loop never waits

mod app_init;
mod keymap;
mod share;
mod terminal_setup;

use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{run_agent_turn, AgentCommand};
use runie_core::{config_reload, AppState, Event as CoreEvent, Snapshot};
use std::{collections::HashMap, io, io::Write, time::Duration};
use tokio::sync::{mpsc, watch};

const ANIM_MS: u64 = 200;

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::PopKeyboardEnhancementFlags,
            crossterm::terminal::LeaveAlternateScreen,
        );
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let _cleanup = Cleanup;
    let terminal = terminal_setup::setup_terminal()?;
    let mut state = AppState::default();
    app_init::apply_trust_on_startup(&mut state);
    app_init::init_scoped_models(&mut state);
    app_init::init_skills(&mut state);
    app_init::init_prompts(&mut state);
    app_init::init_telemetry(&mut state);
    app_init::init_truncation(&mut state);

    // Production-ready startup: if no provider is configured and the
    // mock provider is not enabled, auto-open the login dialog so the
    // user is immediately productive instead of staring at a broken app.
    if state.config.current_provider.is_empty() && !runie_core::provider_registry::is_mock_enabled()
    {
        state.update(CoreEvent::LoginFlowStart);
    }

    let (input_tx, input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = mpsc::channel::<Snapshot>(1);
    let (kb_tx, kb_rx) = watch::channel(state.config.keybindings.clone());

    tokio::spawn(agent_loop(cmd_rx, agent_tx));
    tokio::spawn(input_reader(input_tx.clone(), kb_rx));
    tokio::spawn(render_task(terminal, render_rx));
    tokio::spawn(config_reload::spawn_config_watcher(
        input_tx.clone(),
        config_reload::config_path(),
    ));

    event_loop(
        state, input_rx, agent_rx, cmd_tx, render_tx, input_tx, kb_tx,
    )
    .await
}

async fn render_task(
    mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    mut render_rx: mpsc::Receiver<Snapshot>,
) {
    let mut last_size: Option<(u16, u16)> = None;
    while let Some(snap) = render_rx.recv().await {
        let new_size = terminal
            .size()
            .map(|r| (r.width, r.height))
            .unwrap_or((0, 0));
        // Force a full clear when the terminal dimensions change, so that
        // after a resize to 0×0 (or back from a tiny size) the prompt and
        // widgets are fully re-emitted instead of the diff being a no-op.
        if last_size != Some(new_size) {
            let _ = terminal.clear();
            last_size = Some(new_size);
        }
        let _ = terminal.draw(|f| runie_tui::ui::draw_snapshot(f, &snap));
    }
}

async fn input_reader(
    input_tx: mpsc::Sender<CoreEvent>,
    mut kb_rx: watch::Receiver<HashMap<String, String>>,
) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        let bindings = kb_rx.borrow_and_update().clone();
        if let Some(evt) = keymap::convert_event(&event, &bindings) {
            let is_quit = matches!(&evt, CoreEvent::Quit | CoreEvent::Reset);
            if input_tx.send(evt).await.is_err() {
                break;
            }
            if is_quit {
                break;
            }
        }
    }
}

async fn event_loop(
    mut state: AppState,
    mut input_rx: mpsc::Receiver<CoreEvent>,
    mut agent_rx: mpsc::Receiver<CoreEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: mpsc::Sender<Snapshot>,
    input_tx: mpsc::Sender<CoreEvent>,
    kb_tx: watch::Sender<HashMap<String, String>>,
) -> io::Result<()> {
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

    // Initial draw so the user sees the app immediately, without waiting
    // for the first keyboard event.
    state.ensure_fresh();
    let _ = render_tx.try_send(state.snapshot());

    loop {
        tokio::select! {
            biased;

            Some(evt) = input_rx.recv() => {
                if matches!(evt, CoreEvent::OpenExternalEditor) {
                    let text = state.input.input.clone();
                    let tx = input_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = spawn_external_editor_sync(text, tx);
                    });
                } else if matches!(evt, CoreEvent::ShareSession) {
                    let messages = state.session.messages.clone();
                    let display_name = state.session.session_display_name.clone();
                    let tx = input_tx.clone();
                    tokio::spawn(async move {
                        match share::share_session(&messages, display_name.as_deref()).await {
                            Ok(url) => {
                                let _ = tx.send(CoreEvent::SystemMessage {
                                    content: format!("Shared session: {}", url),
                                }).await;
                            }
                            Err(e) => {
                                let _ = tx.send(CoreEvent::SystemMessage {
                                    content: format!("Could not share session: {}", e),
                                }).await;
                            }
                        }
                    });
                } else if matches!(evt, CoreEvent::Suspend) {
                    #[cfg(unix)]
                    {
                        // Restore terminal to normal before suspending
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::event::PopKeyboardEnhancementFlags,
                            crossterm::terminal::LeaveAlternateScreen,
                        );
                        let _ = crossterm::terminal::disable_raw_mode();

                        // Send SIGTSTP to ourselves
                        let _ = nix::sys::signal::kill(
                            nix::unistd::Pid::this(),
                            nix::sys::signal::Signal::SIGTSTP,
                        );

                        // After resume, restore terminal
                        let _ = crossterm::terminal::enable_raw_mode();
                        let _ = terminal_setup::restore_terminal_graphics(&mut std::io::stdout());

                        // Force redraw
                        state.ensure_fresh();
                        let _ = render_tx.try_send(state.snapshot());
                    }
                } else if let CoreEvent::LoginFlowSubmitKey { .. } = &evt {
                    // Non-blocking: the state update below immediately
                    // transitions to the model selector with default
                    // models. The network fetch runs in the background
                    // and enriches the list when it returns.
                    state.update(evt);
                    if let Some(ref flow) = state.login_flow {
                        let provider = flow.provider.clone();
                        let key = flow.key.clone();
                        if !provider.is_empty() && !key.is_empty() {
                            let tx = input_tx.clone();
                            tokio::spawn(async move {
                                use runie_provider::validate_api_key;
                                use runie_core::provider_registry::find_provider;
                                let result = if let Some(meta) = find_provider(&provider) {
                                    validate_api_key(meta.base_url, &key).await
                                } else {
                                    Err(anyhow::anyhow!("Unknown provider: {}", provider))
                                };
                                match result {
                                    Ok(models) => {
                                        let _ = tx.send(CoreEvent::LoginFlowModelsFetched {
                                            provider,
                                            key,
                                            models,
                                        }).await;
                                    }
                                    Err(e) => {
                                        let _ = tx.send(CoreEvent::LoginFlowValidationFailed {
                                            provider,
                                            key,
                                            error: e.to_string(),
                                        }).await;
                                    }
                                }
                            });
                        }
                    }
                } else if let CoreEvent::SpawnAgent { prompt } = &evt {
                    // Run the subagent in a background task; inject the
                    // result as a system message when done.
                    let provider = state.config.current_provider.clone();
                    let model = state.config.current_model.clone();
                    let thinking = state.config.thinking_level;
                    let read_only = state.config.read_only;
                    let skills = runie_core::skills::build_skills_context(&state.skills);
                    let preview: String = prompt.chars().take(60).collect();
                    let preview = if prompt.chars().count() > 60 {
                        format!("{}…", preview)
                    } else {
                        preview
                    };
                    let tx = input_tx.clone();
                    let prompt = prompt.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = runie_agent::subagent::run_subagent(
                            &prompt, &provider, &model,
                            thinking, read_only, &skills, "", 5,
                        );
                        let msg = match result {
                            Ok(text) => {
                                let snippet: String = text.chars().take(200).collect();
                                let snippet = if text.chars().count() > 200 {
                                    format!("{}…", snippet)
                                } else {
                                    snippet
                                };
                                format!("Subagent \"{}\" → {}", preview, snippet)
                            }
                            Err(e) => format!("Subagent \"{}\" failed: {}", preview, e),
                        };
                        let _ = tx.blocking_send(CoreEvent::SystemMessage { content: msg });
                    });
                } else {
                    let was_submit = matches!(evt, CoreEvent::Submit);
                    let was_followup = matches!(evt, CoreEvent::FollowUp);
                    let was_reload = matches!(evt, CoreEvent::ReloadAll);
                    state.update(evt);
                    if state.should_quit {
                        return Ok(());
                    }
                    if was_reload {
                        let _ = kb_tx.send(state.config.keybindings.clone());
                    }
                    if was_submit || was_followup {
                        spawn_if_queued(&mut state, &cmd_tx).await;
                    }
                }
            }

            Some(evt) = agent_rx.recv() => {
                let was_done = matches!(evt, CoreEvent::AgentDone { .. } | CoreEvent::AgentError { .. });
                state.update(evt);
                if was_done {
                    spawn_if_queued(&mut state, &cmd_tx).await;
                }
            }

            _ = anim.tick() => {
                state.tick_animation();
            }
        }

        state.ensure_fresh();
        let snap = state.snapshot();
        if render_tx.try_send(snap).is_err() {
            // Render task is behind — old snapshot dropped, latest will draw
        }
    }
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        let agent_tx_clone = agent_tx.clone();
        let cmd_id = cmd.id.clone();

        let result = run_agent_turn(
            &cmd,
            |evt| {
                let tx = agent_tx_clone.clone();
                let _ = tx.try_send(evt);
            },
            5,
        )
        .await;

        if let Err(e) = result {
            let _ = agent_tx
                .send(CoreEvent::AgentError {
                    id: cmd_id,
                    message: format!("Agent error: {}", e),
                })
                .await;
        }
    }
}

fn spawn_external_editor_sync(text: String, tx: mpsc::Sender<CoreEvent>) -> io::Result<()> {
    let editor = std::env::var("EDITOR")
        .unwrap_or_else(|_| if cfg!(windows) { "notepad" } else { "vi" }.to_string());

    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(text.as_bytes())?;
    tmp.flush()?;
    let path = tmp.into_temp_path();

    let status = std::process::Command::new(&editor).arg(&path).status()?;

    if status.success() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let _ = handle.block_on(tx.send(CoreEvent::ExternalEditorDone { content }));
        }
    }

    Ok(())
}

async fn spawn_if_queued(state: &mut AppState, cmd_tx: &mpsc::Sender<AgentCommand>) {
    if let Some((content, id)) = state.peek_queue() {
        let content = content.clone();
        let id = id.clone();
        state.pop_queue();
        state.streaming = true;
        state.agent.turn_active = true;
        state.agent.inflight += 1;
        let skills_context = runie_core::skills::build_skills_context(&state.skills);
        let system_prompt = state
            .prompts
            .iter()
            .find(|p| p.name == state.current_prompt)
            .map(|p| p.content.clone())
            .unwrap_or_default();
        let _ = cmd_tx
            .send(AgentCommand {
                content,
                id,
                provider: state.config.current_provider.clone(),
                model: state.config.current_model.clone(),
                thinking_level: state.config.thinking_level,
                read_only: state.config.read_only,
                skills_context,
                system_prompt,
                truncation: runie_agent::truncate::policy_from_section(
                    state.config.truncation.max_lines,
                    state.config.truncation.max_bytes,
                ),
            })
            .await;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn animation_interval_is_200ms() {
        assert_eq!(
            super::ANIM_MS,
            200,
            "ANIM_MS must be 200ms for visible braille spinner, got {}",
            super::ANIM_MS
        );
    }

    #[tokio::test]
    async fn spawn_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(
            state.agent.turn_active,
            "spawn_if_queued must set turn_active"
        );
        assert_eq!(
            state.agent.inflight, 1,
            "spawn_if_queued must increment inflight"
        );
        assert!(
            state.agent.request_queue.is_empty(),
            "Message should be popped from request_queue"
        );

        let cmd = rx.try_recv().expect("Command should be sent to agent");
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.thinking_level, runie_core::model::ThinkingLevel::Off);
        assert_eq!(cmd.system_prompt, "");
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);
    }
}
