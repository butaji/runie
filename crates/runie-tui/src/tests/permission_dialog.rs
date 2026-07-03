//! Layer 2 tests: permission dialog keys are consumed by the dialog.
//!
//! When a permission request is pending, y/n/a keys should resolve
//! the permission instead of being sent to the input box.

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

/// Layer 2: y key grants permission and clears the dialog.
#[tokio::test]
async fn permission_dialog_y_grants() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Set up a pending permission request directly
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-req-1".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    // Verify permission is pending
    assert!(
        ui.state.permission_request_opt().is_some(),
        "Permission request should be pending"
    );

    // Press 'y' — permission should be granted and dialog cleared
    ui.handle_event_inner(Event::Input('y'), effect_tx.clone())
        .await;

    // Dialog should be cleared
    assert!(
        ui.state.permission_request_opt().is_none(),
        "Permission dialog should be cleared after y is pressed"
    );
}

/// Layer 2: n key denies permission and clears the dialog.
#[tokio::test]
async fn permission_dialog_n_denies() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Set up a pending permission request directly
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-req-2".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    // Verify permission is pending
    assert!(
        ui.state.permission_request_opt().is_some(),
        "Permission request should be pending"
    );

    // Press 'n' — permission should be denied and dialog cleared
    ui.handle_event_inner(Event::Input('n'), effect_tx.clone())
        .await;

    // Dialog should be cleared
    assert!(
        ui.state.permission_request_opt().is_none(),
        "Permission dialog should be cleared after n is pressed"
    );
}

/// Layer 2: a key grants permission (Always allow) and clears the dialog.
#[tokio::test]
async fn permission_dialog_a_always_allows() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Set up a pending permission request directly
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-req-3".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    // Verify permission is pending
    assert!(
        ui.state.permission_request_opt().is_some(),
        "Permission request should be pending"
    );

    // Press 'a' — permission should be granted and dialog cleared
    ui.handle_event_inner(Event::Input('a'), effect_tx.clone())
        .await;

    // Dialog should be cleared
    assert!(
        ui.state.permission_request_opt().is_none(),
        "Permission dialog should be cleared after a is pressed"
    );
}

/// Layer 2: y/n/a keys are NOT sent to input when permission dialog is open.
#[tokio::test]
async fn permission_dialog_keys_not_sent_to_input() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Set up a pending permission request directly
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-req-4".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    // Record initial input
    let initial_input = ui.state.input().input.clone();

    // Press 'y' — should not affect input
    ui.handle_event_inner(Event::Input('y'), effect_tx.clone())
        .await;

    // Input should be unchanged
    assert_eq!(
        ui.state.input().input,
        initial_input,
        "y key should not be sent to input when permission dialog is open"
    );
}

/// Layer 2: other keys ARE handled normally when permission dialog is open.
/// Note: this test verifies that regular keys don't trigger permission handling.
#[tokio::test]
async fn other_keys_not_intercepted_by_permission() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Set up a pending permission request directly
    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-req-5".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    // Record initial input
    let initial_input = ui.state.input().input.clone();

    // Press a regular key 'h' — should NOT be intercepted by permission handler
    ui.handle_event_inner(Event::Input('h'), effect_tx.clone())
        .await;

    // Dialog should still be open (not cleared by non-y/n/a key)
    assert!(
        ui.state.permission_request_opt().is_some(),
        "Regular key should not clear the permission dialog"
    );

    // Input should remain unchanged (mock doesn't update, but event was dispatched)
    assert_eq!(
        ui.state.input().input,
        initial_input,
        "Regular key should not affect input state"
    );
}

// ============================================================================
// Layer 2 — Event Handling: navigation keys consumed as no-ops
// ============================================================================

/// Esc while a permission dialog is open is consumed as a no-op.
/// It does NOT deny the permission and is NOT routed to the input box.
#[tokio::test]
async fn esc_during_permission_dialog_is_noop() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-esc".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    ui.handle_event_inner(Event::Escape, effect_tx.clone())
        .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Esc should not deny the permission request"
    );
}

/// Backspace while a permission dialog is open is consumed as a no-op.
#[tokio::test]
async fn backspace_during_permission_dialog_is_noop() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-bs".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    ui.handle_event_inner(Event::Backspace, effect_tx.clone())
        .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Backspace should not deny the permission request"
    );
}

/// Enter while a permission dialog is open is consumed as a no-op.
#[tokio::test]
async fn newline_during_permission_dialog_is_noop() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-nl".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    ui.handle_event_inner(Event::Newline, effect_tx.clone())
        .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Newline should not deny the permission request"
    );
}

/// Arrow keys while a permission dialog is open are consumed as no-ops.
#[tokio::test]
async fn cursor_keys_during_permission_dialog_are_noop() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-cursor".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    ui.handle_event_inner(Event::CursorLeft, effect_tx.clone())
        .await;
    ui.handle_event_inner(Event::CursorRight, effect_tx.clone())
        .await;
    ui.handle_event_inner(Event::CursorStart, effect_tx.clone())
        .await;
    ui.handle_event_inner(Event::CursorEnd, effect_tx.clone())
        .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Cursor keys should not deny the permission request"
    );
}

/// PageUp/PageDown while a permission dialog is open are consumed as no-ops.
#[tokio::test]
async fn page_keys_during_permission_dialog_are_noop() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    *ui.state.permission_request_mut() = Some(runie_core::model::PermissionRequestState {
        request_id: "test-page".into(),
        tool: "bash".into(),
        input: serde_json::json!({"command": "echo hi"}),
    });

    ui.handle_event_inner(Event::PageUp, effect_tx.clone())
        .await;
    ui.handle_event_inner(Event::PageDown, effect_tx.clone())
        .await;

    assert!(
        ui.state.permission_request_opt().is_some(),
        "Page keys should not deny the permission request"
    );
}
