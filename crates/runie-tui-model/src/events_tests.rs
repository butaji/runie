use super::{
    event_projection_scope, is_actor_feed_event, project_event, scrollback_messages_for_event,
    status_messages_for_event, EventProjectionScope,
};
use runie_core::types::{AgentEvent, AgentMessage, AssistantMessage, AssistantMessageEvent};

#[test]
fn actor_feed_scope_excludes_transcript_messages() {
    assert!(!is_actor_feed_event(&AgentEvent::TurnStart));
    assert!(!is_actor_feed_event(&AgentEvent::AgentEnd {
        messages: vec![]
    }));
    assert!(is_actor_feed_event(&AgentEvent::Reset));
    assert!(is_actor_feed_event(&AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::GrokNight,
    }));
}

#[test]
fn event_scopes_share_multi_owner_events_without_broadening_feed_admission() {
    let theme = event_projection_scope(&AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::GrokNight,
    });
    assert!(theme.contains(EventProjectionScope::FEED));
    assert!(theme.contains(EventProjectionScope::STATUS));
    let message = event_projection_scope(&AgentEvent::TurnStart);
    assert!(message.contains(EventProjectionScope::STATUS));
    assert!(!message.contains(EventProjectionScope::FEED));
    let session = event_projection_scope(&AgentEvent::SessionNameChanged {
        name: "demo".into(),
    });
    assert!(session.contains(EventProjectionScope::SESSION));
    assert!(!session.contains(EventProjectionScope::TRANSCRIPT));
}

#[test]
fn scrollback_projection_is_model_owned_for_feed_events() {
    let messages = scrollback_messages_for_event(&AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({"cmd": "pwd"}),
    });
    assert_eq!(messages.len(), 4);
    assert_eq!(
        messages[0],
        super::ScrollbackMsg::SetToolName("call-1".into(), "bash".into())
    );
    assert_eq!(
        messages[1],
        super::ScrollbackMsg::SetToolArgs("call-1".into(), serde_json::json!({"cmd": "pwd"}))
    );
    assert_eq!(
        messages[2],
        super::ScrollbackMsg::ActivityToolStart("bash".into())
    );
}

#[test]
fn thinking_duration_is_delivered_to_the_status_actor() {
    let messages = status_messages_for_event(&AgentEvent::MessageUpdate {
        message: AgentMessage::Assistant(AssistantMessage::default()),
        event: AssistantMessageEvent::ThinkingEnd {
            index: 0,
            content: "reasoning".into(),
            partial: AssistantMessage::default(),
            elapsed_ms: Some(900),
        },
    });
    assert_eq!(
        messages,
        vec![super::StatusMsg::SetThinkingElapsed(Some(900))]
    );
}

#[test]
fn event_projection_keeps_multi_actor_fanout_in_one_value() {
    let projection = project_event(&AgentEvent::ThemeChanged {
        theme: runie_core::types::ThemeKind::GrokDay,
    });
    assert!(projection.scope.contains(EventProjectionScope::FEED));
    assert!(projection.scope.contains(EventProjectionScope::STATUS));
    assert_eq!(
        projection.feed,
        vec![super::ScrollbackMsg::SetTheme(
            runie_core::types::ThemeKind::GrokDay
        )]
    );
    assert_eq!(
        projection.status,
        vec![super::StatusMsg::SetTheme(
            runie_core::types::ThemeKind::GrokDay
        )]
    );
    assert!(projection.ui.is_empty());
}
