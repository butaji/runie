//! Layer 2 tests: `UiActor` routes `Event::Input` through `InputActor`.
//!
//! Ensures that input events go through the InputActor → InputChanged path,
//! not direct `AppState.input` mutation.

use std::sync::Arc;

use runie_core::actors::leader::LeaderAgentCmd;
use runie_core::actors::leader::LeaderAgentHandle;
use runie_core::Event;

/// Minimal mock agent handle for testing.
struct MockAgentHandle;

impl LeaderAgentHandle for MockAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

/// Layer 2: `Event::Input(c)` sent to UiActor produces exactly one
/// `InputChanged` event (proving InputActor received the InsertChar message).
#[tokio::test]
async fn input_event_routes_to_input_actor() {
    // Build a LeaderHandle with all real actors (including InputActor).
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    // Subscribe before UiActor starts so we capture all events.
    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    // Second subscription to observe events.
    let mut sub = bus.subscribe();

    // Run UiActor in a background task.
    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // Advance virtual time to let actor start.
    let _guard = runie_testing::TestTimeGuard::new().expect("should support time pausing");
    runie_testing::TestTimeGuard::advance(std::time::Duration::from_millis(50)).await;

    // Send Input('h') through the submit channel.
    submit_tx
        .send(Event::Input('h'))
        .await
        .expect("submit channel open");

    // Wait for InputChanged event (InputActor emits it in response to InsertChar).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut found_input_changed = false;
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(evt)) => {
                if matches!(evt, Event::InputChanged { state } if state.input == "h") {
                    found_input_changed = true;
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        found_input_changed,
        "Expected InputChanged event with 'h' after Event::Input('h')"
    );

    // Quit to shut down the actor.
    submit_tx.send(Event::Quit).await.expect("submit open");
    let _ = ui_handle.await;

    leader.shutdown().await;
}

/// Layer 2: Multiple characters accumulate correctly through InputActor.
#[tokio::test]
async fn input_accumulates_via_input_actor() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    let mut sub = bus.subscribe();

    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // Advance virtual time to let actor start.
    let _guard = runie_testing::TestTimeGuard::new().expect("should support time pausing");
    runie_testing::TestTimeGuard::advance(std::time::Duration::from_millis(50)).await;

    // Type "hi" character by character.
    submit_tx
        .send(Event::Input('h'))
        .await
        .expect("submit open");
    submit_tx
        .send(Event::Input('i'))
        .await
        .expect("submit open");

    // Wait for the final InputChanged event.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut final_input = String::new();
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(evt)) => {
                if let Event::InputChanged { state } = evt {
                    final_input = state.input.clone();
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert_eq!(
        final_input, "hi",
        "Input should accumulate to 'hi' via InputActor routing"
    );

    // Quit to shut down the actor.
    submit_tx.send(Event::Quit).await.expect("submit open");
    let _ = ui_handle.await;

    leader.shutdown().await;
}

/// Layer 2: When a dialog is open, input events are applied to the dialog form
/// on state instead of being routed to InputActor. This is the regression path
/// that broke onboarding typing/arrows: the canonical router sent everything to
/// InputActor, which only mutates the chat input box.
#[tokio::test]
async fn input_event_routes_to_dialog_when_open() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    // Open the onboarding login flow key-input panel.
    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());
    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    let panel_id = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id.clone());
    assert_eq!(
        panel_id,
        Some("login-key".to_string()),
        "setup should open key input panel"
    );

    let bus_rx = bus.subscribe();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Type while the dialog has focus.
    ui.handle_event(Event::Input('x'), effect_tx.clone()).await;
    ui.handle_event(Event::Input('y'), effect_tx.clone()).await;
    ui.handle_event(Event::Backspace, effect_tx.clone()).await;

    // The dialog form field should have received the input.
    let panel = ui
        .state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .expect("key input panel should still be open");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"x".to_string()),
        "typed characters should land in dialog form, not chat input"
    );

    // The chat input box should remain empty because the events were not
    // routed to InputActor while the dialog was open.
    assert!(
        ui.state.input().input().is_empty(),
        "chat input should stay empty while dialog has focus"
    );

    leader.shutdown().await;
}

/// Layer 2: When a dialog is open, Enter (Event::Submit) is applied to the
/// dialog state instead of being captured by the chat input box. This is the
/// regression path that broke the onboarding login flow after API key entry:
/// Submit was sent to InputActor with empty chat input and silently dropped.
#[tokio::test]
async fn submit_event_routes_to_dialog_when_open() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    // Open the onboarding login flow provider picker.
    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());
    state.update(Event::Start);
    let panel_id = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id.clone());
    assert_eq!(
        panel_id,
        Some("login-provider".to_string()),
        "setup should open provider picker"
    );

    let bus_rx = bus.subscribe();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Press Enter while the provider picker has focus.
    ui.handle_event(Event::Submit, effect_tx.clone()).await;

    // The dialog should advance to the API key input panel for the selected
    // provider, proving Submit was routed to the dialog and not the chat input.
    let panel_id = ui
        .state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id.clone());
    assert_eq!(
        panel_id,
        Some("login-key".to_string()),
        "Enter should activate the selected provider and open the key input panel"
    );

    // The chat input box should remain empty because Submit was not routed to
    // InputActor while the dialog was open.
    assert!(
        ui.state.input().input().is_empty(),
        "chat input should stay empty while dialog has focus"
    );

    leader.shutdown().await;
}

/// Layer 2: Submitting the onboarding API-key form publishes `Event::SubmitKey`
/// on the event bus so UiActor can dispatch the async validation effect.
///
/// Regression: the form previously called `state.update(SubmitKey)` silently,
/// so `effects::dispatch` never saw the event and the "Verifying ..." panel
/// stayed stuck forever.
#[tokio::test]
async fn login_form_submit_publishes_submit_key_event() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    // Open the onboarding login flow key-input panel.
    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());
    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    let panel_id = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
        .map(|p| p.id.clone());
    assert_eq!(
        panel_id,
        Some("login-key".to_string()),
        "setup should open key input panel"
    );

    let bus_rx = bus.subscribe();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Subscribe before typing so we can observe the published event.
    let mut sub = bus.subscribe();

    // Type an API key into the form field, then press Enter.
    for c in "sk-test".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    ui.handle_event(Event::Submit, effect_tx.clone()).await;

    // The form should have closed and emitted SubmitKey on the bus.
    let evt = sub
        .try_recv()
        .expect("SubmitKey should be published on the bus");
    assert!(
        matches!(
            &evt,
            Event::SubmitKey { provider, key }
                if provider == "minimax" && key == "sk-test"
        ),
        "expected SubmitKey for minimax, got: {:?}",
        evt
    );

    leader.shutdown().await;
}

/// Build a `UiActor` wired to real actors but driven manually by tests.
async fn manual_ui_actor() -> (
    crate::ui_actor::UiActor,
    tokio::sync::mpsc::Sender<Event>,
    runie_core::actors::leader::LeaderHandle,
) {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let ui = crate::ui_actor::UiActor::with_agent_handle(
        state,
        crate::ui_actor_agent_handles::AgentHandleBox::Leader(agent_handle),
        Some(leader.turn.clone()),
        Some(leader.input.clone()),
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    (ui, effect_tx, leader)
}

/// Regression: the '/' autocomplete trigger must open the command palette
/// synchronously, before the next key event is processed. Otherwise rapid
/// typing can leave the palette filter empty and cause Enter to run the
/// first palette item (/approve) instead of the intended command.
#[tokio::test]
async fn slash_opens_command_palette_synchronously() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;
    assert!(ui.state.open_dialog().is_none());

    ui.handle_event(Event::Input('/'), effect_tx).await;

    assert!(
        ui.state.open_dialog().is_some(),
        "'/' should open the command palette synchronously"
    );

    leader.shutdown().await;
}

/// Regression: '/' typed after existing text must NOT open the command
/// palette or destroy the typed text. The UiActor caches a `prev_input`
/// snapshot for the asynchronous autocomplete trigger; when that snapshot
/// goes stale (stays empty), `detect_autocomplete_trigger` fires on any '/'
/// at end of input — opening the palette and clearing the input box, which
/// makes it impossible to type paths like `src/main.rs`.
#[tokio::test]
async fn slash_after_text_does_not_open_palette() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;
    assert!(ui.state.open_dialog().is_none());

    // Simulate realistic typing: each key goes to the InputActor and its
    // InputChanged round-trip updates the UiActor's input projection.
    let mut typed = String::new();
    for c in "src".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
        typed.push(c);
        let mut istate = runie_core::InputState::default();
        istate.input = typed.clone();
        istate.cursor_pos = typed.len();
        ui.handle_event(
            Event::InputChanged {
                state: Box::new(istate),
            },
            effect_tx.clone(),
        )
        .await;
    }
    assert_eq!(ui.state.input().input, "src");

    // Now type '/' at the end of non-empty input.
    ui.handle_event(Event::Input('/'), effect_tx.clone()).await;
    typed.push('/');
    let mut istate = runie_core::InputState::default();
    istate.input = typed.clone();
    istate.cursor_pos = typed.len();
    ui.handle_event(
        Event::InputChanged {
            state: Box::new(istate),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.open_dialog().is_none(),
        "'/' after text must not open the command palette"
    );
    assert_eq!(
        ui.state.input().input, "src/",
        "'/' must be inserted literally, input must not be cleared"
    );

    leader.shutdown().await;
}

/// Regression: '/' typed after a space inside a message (e.g. "check /tmp")
/// must NOT open the command palette or eat the typed text. Commands only
/// make sense as the whole input; a slash after any typed content is a path
/// separator, not a command trigger.
#[tokio::test]
async fn slash_after_space_mid_input_does_not_open_palette() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;
    assert!(ui.state.open_dialog().is_none());

    // Realistic typing with InputChanged round-trips, ending in a space.
    let mut typed = String::new();
    for c in "check ".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
        typed.push(c);
        let mut istate = runie_core::InputState::default();
        istate.input = typed.clone();
        istate.cursor_pos = typed.len();
        ui.handle_event(
            Event::InputChanged {
                state: Box::new(istate),
            },
            effect_tx.clone(),
        )
        .await;
    }
    assert_eq!(ui.state.input().input, "check ");

    // Now type '/' — with the trailing space the old trigger treated this as
    // a fresh command token and opened the palette, destroying "check ".
    ui.handle_event(Event::Input('/'), effect_tx.clone()).await;
    typed.push('/');
    let mut istate = runie_core::InputState::default();
    istate.input = typed.clone();
    istate.cursor_pos = typed.len();
    ui.handle_event(
        Event::InputChanged {
            state: Box::new(istate),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.open_dialog().is_none(),
        "'/' after a space mid-input must not open the command palette"
    );
    assert_eq!(
        ui.state.input().input, "check /",
        "'/' must be inserted literally after a space"
    );

    leader.shutdown().await;
}

/// Regression: '/' typed immediately after fast-entered text (no InputChanged
/// round-trips processed yet, as happens with paste-speed typing) must NOT
/// open the command palette. The AppState projection lags the InputActor by
/// one round-trip, so the trigger check must use the optimistic pending
/// character mirror instead of the projection alone.
#[tokio::test]
async fn slash_after_fast_typed_text_does_not_open_palette() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;
    assert!(ui.state.open_dialog().is_none());

    // No InputChanged events pumped: the projection stays empty while the
    // InputActor (and the UiActor's pending mirror) hold the real text.
    for c in "src".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    ui.handle_event(Event::Input('/'), effect_tx.clone()).await;

    assert!(
        ui.state.open_dialog().is_none(),
        "'/' after fast-typed text must not open the command palette"
    );

    leader.shutdown().await;
}

/// Regression: submitting immediately after fast typing must not lose the
/// characters whose InputChanged echo has not been processed yet. Submit
/// used to read only the (lagging) AppState projection, so a rapid
/// "quickbrownfox"+Enter submitted just "quickb".
#[tokio::test]
async fn submit_after_fast_typing_keeps_full_content() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    for c in "quickbrownfox".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    ui.handle_event(Event::Submit, effect_tx.clone()).await;
    // Deliver the cleared-input echo the InputActor emits on submit; this
    // triggers dispatch of the pending submit content.
    ui.handle_event(
        Event::InputChanged {
            state: Box::new(runie_core::InputState::default()),
        },
        effect_tx.clone(),
    )
    .await;

    // Dispatch routes user messages through the async TurnActor (unobservable
    // in this manual harness), but it synchronously records the submitted
    // content in the input history — so a complete entry here proves the
    // full fast-typed text reached the submit path.
    assert!(
        ui.state
            .input()
            .input_history
            .iter()
            .any(|h| h == "quickbrownfox"),
        "full fast-typed content must be submitted, history: {:?}",
        ui.state.input().input_history
    );

    leader.shutdown().await;
}

/// Regression: Up on an EMPTY chat input must scroll the feed in production
/// routing too, not only in the core `update` path.
///
/// UiActor routes HistoryPrev/HistoryNext straight to the InputActor
/// (`route_to_input_actor`), bypassing the core history-nav mode dispatch.
/// Terminals with "alternate scroll" (iTerm2, kitty, WezTerm) translate
/// mouse-wheel ticks into arrow keys when the app does not capture the mouse
/// (runie keeps native selection), so empty-input arrows must scroll the
/// feed instead of cycling prompt history.
#[tokio::test]
async fn up_on_empty_input_scrolls_feed_in_production() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    // Seed a message so the feed has scrollback.
    ui.state.submit_user_message("hello".to_string());
    assert_eq!(ui.state.view().scroll, 0);

    ui.handle_event(Event::HistoryPrev, effect_tx.clone()).await;

    assert_eq!(
        ui.state.view().scroll, 1,
        "Up on empty input must scroll the feed in production routing"
    );
    assert!(
        ui.state.input().input.is_empty(),
        "Up on empty input must not recall history into the input box"
    );

    leader.shutdown().await;
}

/// Down on an EMPTY chat input scrolls the feed toward newer content.
#[tokio::test]
async fn down_on_empty_input_scrolls_feed_in_production() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    ui.state.submit_user_message("hello".to_string());
    ui.state.view_mut().scroll = 3;

    ui.handle_event(Event::HistoryNext, effect_tx.clone()).await;

    assert_eq!(
        ui.state.view().scroll, 2,
        "Down on empty input must scroll the feed down in production routing"
    );

    leader.shutdown().await;
}

/// With text in the input, Up keeps history routing (no feed scroll) — the
/// scroll interception must only fire for an empty input box.
#[tokio::test]
async fn up_with_text_does_not_scroll_feed_in_production() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    ui.state.submit_user_message("hello".to_string());
    assert_eq!(ui.state.view().scroll, 0);

    // Type 'x' and deliver its InputChanged echo so the projection is fresh.
    ui.handle_event(Event::Input('x'), effect_tx.clone()).await;
    let mut istate = runie_core::InputState::default();
    istate.input = "x".to_string();
    istate.cursor_pos = 1;
    ui.handle_event(
        Event::InputChanged {
            state: Box::new(istate),
        },
        effect_tx.clone(),
    )
    .await;

    ui.handle_event(Event::HistoryPrev, effect_tx.clone()).await;

    assert_eq!(
        ui.state.view().scroll, 0,
        "Up with text in the input must not scroll the feed"
    );

    leader.shutdown().await;
}

/// Race guard: fast typing whose InputChanged echo has not been processed
/// yet leaves the AppState projection empty while the optimistic pending
/// mirror holds the text. Up in that window must still route to the
/// InputActor (history), not scroll the feed.
#[tokio::test]
async fn up_after_fast_typing_does_not_scroll_feed_in_production() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    ui.state.submit_user_message("hello".to_string());
    assert_eq!(ui.state.view().scroll, 0);

    // No InputChanged events pumped: projection stays empty, pending mirror
    // holds the typed text.
    for c in "src".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    assert!(ui.state.input().input.is_empty());

    ui.handle_event(Event::HistoryPrev, effect_tx.clone()).await;

    assert_eq!(
        ui.state.view().scroll, 0,
        "Up after fast-typed (pending) text must route to history, not scroll"
    );

    leader.shutdown().await;
}

/// Regression: typing `/model` as a continuous sequence must open the model
/// selector, even when no InputChanged round-trips have been processed.
#[tokio::test]
async fn slash_model_selects_model_synchronously() {
    use runie_core::commands::{DialogKind, DialogState};

    std::env::remove_var("RUNIE_MOCK");
    std::env::remove_var("RUNIE_MOCK_DELAY");
    runie_core::provider::set_mock_enabled(true);

    let (mut ui, effect_tx, leader) = manual_ui_actor().await;

    for c in "/model".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    ui.handle_event(Event::Submit, effect_tx.clone()).await;

    runie_core::provider::set_mock_enabled(false);

    let msgs: Vec<String> = ui.state.session().messages.iter().map(|m| m.content()).collect();
    assert!(
        !msgs.iter().any(|m| m.contains("No pending edits to approve")),
        "/model must not run /approve, messages: {:?}",
        msgs
    );
    assert!(
        matches!(
            ui.state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::ModelSelector,
                panels: _,
            })
        ),
        "/model should open the model selector, got {:?}",
        ui.state.open_dialog()
    );

    leader.shutdown().await;
}

/// Regression: typing a prefix, then '@', and picking a file must keep the
/// typed prefix ("read @path") — and the authoritative InputActor must hold
/// the same text so the next keystroke does not clobber the box.
///
/// Production bug: the picker opens with a Clear to the InputActor; the
/// resulting InputChanged echo wholesale-replaces the projection InputState
/// and wiped the projection-only file-picker backup, so the pick inserted
/// the bare filename over the whole input.
#[tokio::test]
async fn at_file_pick_preserves_prefix_and_syncs_input_actor() {
    let (mut ui, effect_tx, leader) = manual_ui_actor().await;
    crate::tests::core::inject_mock_file_entries(&mut ui.state);
    let mut sub = leader.event_bus().subscribe();

    // Type "read " (routed to the InputActor), then '@' — the trigger opens
    // the picker synchronously and clears the InputActor.
    for c in "read ".chars() {
        ui.handle_event(Event::Input(c), effect_tx.clone()).await;
    }
    ui.handle_event(Event::Input('@'), effect_tx.clone()).await;
    assert!(ui.state.open_dialog().is_some(), "picker should open");

    // Production ordering: the Clear echo arrives AFTER the picker opened.
    ui.handle_event(
        Event::InputChanged {
            state: Box::default(),
        },
        effect_tx.clone(),
    )
    .await;

    // Pick the first entry.
    ui.handle_event(Event::Submit, effect_tx.clone()).await;
    assert!(ui.state.open_dialog().is_none(), "picker should close");
    let final_text = ui.state.input().input.clone();
    assert!(
        final_text.starts_with("read @"),
        "pick must preserve the typed prefix, got: {final_text:?}"
    );

    // The authoritative InputActor must hold the same text; its InputChanged
    // echo proves the sync (without it, the next keystroke clobbers the box).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut synced = false;
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(Event::InputChanged { state })) if state.input == final_text => {
                synced = true;
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        synced,
        "InputActor should echo the picked text {final_text:?} (input sync)"
    );

    leader.shutdown().await;
}
