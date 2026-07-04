//! Layer 2 tests: permission dialog is now a hosted panel.
//!
//! Permission decisions are made by selecting an action in the hosted form
//! panel, which emits PermissionAllow / PermissionDeny / PermissionAlwaysAllow
//! events. UiActor resolves the pending request and clears the UI state.

use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

use crate::ui_actor::UiActor;
use crate::ui_actor_agent_handles::AgentHandleBox;

/// Minimal mock agent handle for testing.
struct MockAgentHandle {
    #[allow(dead_code)]
    run_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockAgentHandle {
    fn new() -> (Arc<Self>, tokio::sync::mpsc::Receiver<LeaderAgentCmd>) {
        let (_tx, rx) = tokio::sync::mpsc::channel(16);
        (
            Arc::new(Self {
                run_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }),
            rx,
        )
    }
}

impl LeaderAgentHandle for MockAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        self.run_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

fn make_ui_actor() -> (UiActor, Arc<MockAgentHandle>) {
    let (agent, _rx) = MockAgentHandle::new();
    let agent_arc = Arc::new(MockAgentHandle {
        run_count: agent.run_count.clone(),
    });
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(agent_handle),
        None,
        None,
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );
    (ui, agent_arc)
}

fn set_permission_request(ui: &mut UiActor, request_id: &str) {
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: request_id.into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });
}

async fn open_permission_request(ui: &mut UiActor, effect_tx: &tokio::sync::mpsc::Sender<Event>) {
    ui.handle_event_inner(
        Event::PermissionRequest {
            request_id: "test-req".into(),
            tool: "bash".into(),
            input: serde_json::json!({"command": "echo hi"}),
        },
        effect_tx.clone(),
    )
    .await;
}

fn selected_index(state: &runie_core::AppState) -> usize {
    state
        .open_dialog()
        .as_ref()
        .expect("dialog should be open")
        .panel_stack()
        .expect("panel stack")
        .current()
        .expect("panel")
        .selected
}

/// Layer 2: PermissionAllow event grants permission and clears the request state.
#[tokio::test]
async fn permission_allow_event_clears_request() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    set_permission_request(&mut ui, "test-allow");

    ui.handle_event_inner(
        Event::PermissionAllow {
            request_id: "test-allow".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.permission_request_opt().is_none(),
        "PermissionAllow should clear the request state"
    );
}

/// Layer 2: PermissionDeny event denies permission and clears the request state.
#[tokio::test]
async fn permission_deny_event_clears_request() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    set_permission_request(&mut ui, "test-deny");

    ui.handle_event_inner(
        Event::PermissionDeny {
            request_id: "test-deny".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.permission_request_opt().is_none(),
        "PermissionDeny should clear the request state"
    );
}

/// Layer 2: PermissionAlwaysAllow event grants permission and clears the request state.
#[tokio::test]
async fn permission_always_allow_event_clears_request() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    set_permission_request(&mut ui, "test-always");

    ui.handle_event_inner(
        Event::PermissionAlwaysAllow {
            request_id: "test-always".into(),
            tool: "bash".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.permission_request_opt().is_none(),
        "PermissionAlwaysAllow should clear the request state"
    );
}

/// Layer 2: permission dialog events for a different request id are ignored.
#[tokio::test]
async fn permission_event_wrong_id_is_ignored() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    set_permission_request(&mut ui, "test-a");

    ui.handle_event_inner(
        Event::PermissionAllow {
            request_id: "test-b".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Mismatched request id should not clear the request state"
    );
}

// ============================================================================
// Layer 2 — Hosted permission panel receives navigation and activation keys
// ============================================================================

#[tokio::test]
async fn hosted_permission_dialog_opens_on_request() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    assert!(ui.state.permission_request_opt().is_some());
    assert!(ui.state.open_dialog().is_some(), "hosted dialog should be open");
}

#[tokio::test]
async fn hosted_permission_dialog_arrow_keys_navigate() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    assert_eq!({ selected_index(&ui.state) }, 0);

    ui.handle_event_inner(Event::HistoryNext, effect_tx.clone()).await;
    assert_eq!({ selected_index(&ui.state) }, 1);

    ui.handle_event_inner(Event::HistoryPrev, effect_tx.clone()).await;
    assert_eq!({ selected_index(&ui.state) }, 0);
}

#[tokio::test]
async fn hosted_permission_dialog_enter_activates_allow() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    ui.handle_event_inner(Event::Submit, effect_tx.clone()).await;

    assert!(ui.state.open_dialog().is_none(), "dialog should close");
    assert!(
        ui.state.permission_request_opt().is_none(),
        "request should be resolved"
    );
}

#[tokio::test]
async fn hosted_permission_dialog_down_enter_activates_deny() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    ui.handle_event_inner(Event::HistoryNext, effect_tx.clone()).await;
    ui.handle_event_inner(Event::Submit, effect_tx.clone()).await;

    assert!(ui.state.open_dialog().is_none(), "dialog should close");
    assert!(
        ui.state.permission_request_opt().is_none(),
        "request should be resolved"
    );
}

#[tokio::test]
async fn hosted_permission_dialog_esc_keeps_dialog_open() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    ui.handle_event_inner(Event::Escape, effect_tx.clone()).await;

    assert!(ui.state.open_dialog().is_some(), "dialog should stay open");
    assert!(
        ui.state.permission_request_opt().is_some(),
        "request should remain pending"
    );
}

#[tokio::test]
async fn hosted_permission_dialog_tab_navigates_buttons() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    assert_eq!({ selected_index(&ui.state) }, 0);

    ui.handle_event_inner(Event::Input('\t'), effect_tx.clone()).await;
    assert_eq!({ selected_index(&ui.state) }, 1);

    ui.handle_event_inner(Event::Input('\t'), effect_tx.clone()).await;
    assert_eq!({ selected_index(&ui.state) }, 2);

    ui.handle_event_inner(Event::Input('\t'), effect_tx.clone()).await;
    assert_eq!({ selected_index(&ui.state) }, 0);
}

#[tokio::test]
async fn hosted_permission_dialog_shift_tab_navigates_buttons() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    open_permission_request(&mut ui, &effect_tx).await;

    assert_eq!({ selected_index(&ui.state) }, 0);

    ui.handle_event_inner(Event::CycleThinkingLevel, effect_tx.clone())
        .await;
    assert_eq!({ selected_index(&ui.state) }, 2);
}
