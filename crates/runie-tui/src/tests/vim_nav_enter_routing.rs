//! Production-path tests: Enter in feed navigation must expand the selected
//! collapsible (thought) post — and keep its legacy global-toggle fallback
//! on non-collapsible posts.
//!
//! Regression covered: `UiActor::handle_input_event` only routed
//! Input/HistoryPrev/HistoryNext/Backspace through `apply_event` while in vim
//! nav mode. Enter (`Event::Submit`) fell through to the chat-submit path, so
//! the per-post expand arm in `AppState::handle_vim_nav_event` was dead code
//! in production. Every earlier test drove `state.update()` directly and
//! never exercised the production routing.

use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::{AppState, ChatMessage, Event, Part, Role};

use crate::ui_actor::UiActor;
use crate::ui_actor_agent_handles::AgentHandleBox;

/// Minimal mock agent handle (same pattern as tests/quit_shortcut.rs).
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

fn make_ui_actor() -> UiActor {
    let agent_handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(
        MockAgentHandle,
    ));
    let state = AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    // No turn/input handles: the test drives the full UiActor event path and
    // asserts on AppState, which is exactly what production routing updates.
    UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(agent_handle),
        None,
        None,
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    )
}

fn push_message(ui: &mut UiActor, role: Role, content: &str, timestamp: f64, id: &str) {
    ui.state.session.messages.push(ChatMessage {
        role,
        parts: vec![Part::Text {
            content: content.into(),
        }],
        timestamp,
        id: id.to_string(),
        ..Default::default()
    });
    ui.state.refresh_after_message_change();
}

#[tokio::test]
async fn enter_in_feed_nav_expands_thought_via_production_routing() {
    let mut ui = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    push_message(&mut ui, Role::User, "hi", 0.0, "u1");
    push_message(&mut ui, Role::Thought, "line1\nline2\nline3", 1.0, "t1");

    // Posts: [user(0), thought(1)]. The thought is summarized by default.
    let thought_idx = 1;
    assert!(
        !ui.state.view().expanded_posts.contains(&thought_idx),
        "thought must start collapsed"
    );

    // Esc enters feed nav and selects the bottom post (the thought), exactly
    // as in production.
    ui.handle_event_inner(Event::DialogBack, effect_tx.clone())
        .await;
    assert!(ui.state.view().vim_nav_mode, "Esc should enter feed navigation");
    assert_eq!(
        ui.state.view().selected_post,
        Some(thought_idx),
        "Esc should select the bottom post"
    );

    // Enter via the PRODUCTION routing must expand the selected thought post.
    ui.handle_event_inner(Event::Submit, effect_tx.clone()).await;
    assert!(
        ui.state.view().expanded_posts.contains(&thought_idx),
        "Enter in feed nav must expand the selected thought post through the \
         production event routing (UiActor::handle_input_event)"
    );
    assert!(
        ui.state.view().vim_nav_mode,
        "expanding a post must keep feed navigation active"
    );

    // A second Enter collapses the thought back to its one-line summary.
    ui.handle_event_inner(Event::Submit, effect_tx.clone()).await;
    assert!(
        !ui.state.view().expanded_posts.contains(&thought_idx),
        "second Enter should collapse the thought back to its summary"
    );
}

#[tokio::test]
async fn enter_on_non_collapsible_post_keeps_global_toggle_fallback() {
    let mut ui = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Only a user message: no collapsible (thought) posts in the feed.
    push_message(&mut ui, Role::User, "hi", 0.0, "u1");

    ui.handle_event_inner(Event::DialogBack, effect_tx.clone())
        .await;
    assert!(ui.state.view().vim_nav_mode);

    // Legacy behavior: Enter on a non-collapsible post toggles the global
    // expand/collapse flag (same as Ctrl+O) — now through production routing.
    assert!(!ui.state.view().all_collapsed);
    ui.handle_event_inner(Event::Submit, effect_tx.clone()).await;
    assert!(
        ui.state.view().all_collapsed,
        "Enter on a non-collapsible post should toggle global collapse"
    );
}
