use super::*;
use crate::types::{AssistantMessage, StopReason, UserContent, UserMessage};

#[tokio::test]
async fn push_message_visible_in_snapshot() {
    let actor = AgentStateActor::new();
    actor
        .push_message(AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: "hi".into() }],
            timestamp: 1,
        }))
        .await;
    actor.sync().await;
    let snap = actor.snapshot();
    assert_eq!(snap.messages.len(), 1);
}

#[tokio::test]
async fn replace_messages_acknowledges_before_returning() {
    let actor = AgentStateActor::new();
    actor
        .replace_messages(vec![AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: "restored".into(),
            }],
            timestamp: 1,
        })])
        .await;
    assert_eq!(actor.snapshot().messages.len(), 1);
    assert_eq!(actor.snapshot().messages[0].timestamp(), 1);
}

#[tokio::test]
async fn model_changed_event_updates_the_owned_model_projection() {
    let actor = AgentStateActor::new();
    let model = Model {
        id: "model-1".into(),
        context_window: 42_000,
        ..Model::default()
    };
    actor
        .apply_event(&AgentEvent::ModelChanged {
            model: model.clone(),
        })
        .await;
    assert_eq!(actor.snapshot().model, model);
}

#[tokio::test]
async fn reset_clears_state() {
    let actor = AgentStateActor::new();
    actor.set_system_prompt("sys".into()).await;
    actor.mark_streaming(true).await;
    actor.reset().await;
    actor.sync().await;
    let snap = actor.snapshot();
    assert_eq!(snap.system_prompt, "");
    assert!(!snap.is_streaming);
}

#[tokio::test]
async fn workflow_lifecycle_is_owned_by_core_snapshot() {
    let actor = AgentStateActor::new();
    actor
        .apply_event(&AgentEvent::WorkflowStarted {
            run_id: "wf-1".into(),
            name: "release".into(),
            objective: "ship it".into(),
        })
        .await;
    actor
        .apply_event(&AgentEvent::WorkflowProgress {
            run_id: "wf-1".into(),
            phase: "tests".into(),
            state: "active".into(),
            active_agents: 2,
        })
        .await;
    actor
        .apply_event(&AgentEvent::WorkflowFinished {
            run_id: "wf-1".into(),
            status: "done".into(),
            elapsed_ms: Some(1_200),
        })
        .await;
    actor.sync().await;
    let workflow = actor.snapshot().workflows.remove("wf-1").unwrap();
    assert_eq!(workflow.name, "release");
    assert_eq!(workflow.phase.as_deref(), Some("tests"));
    assert_eq!(workflow.active_agents, 2);
    assert_eq!(workflow.status, "done");
    assert_eq!(workflow.elapsed_ms, Some(1_200));
}

#[tokio::test]
async fn background_work_lifecycle_is_owned_by_core_snapshot() {
    let actor = AgentStateActor::new();
    actor
        .apply_event(&AgentEvent::BackgroundWorkStarted {
            work_id: "bg-1".into(),
            description: "index files".into(),
            background: true,
        })
        .await;
    actor
        .apply_event(&AgentEvent::BackgroundWorkProgress {
            work_id: "bg-1".into(),
            description: "index files".into(),
            activity: "scanning src".into(),
        })
        .await;
    actor
        .apply_event(&AgentEvent::BackgroundWorkFinished {
            work_id: "bg-1".into(),
            description: "index files".into(),
            is_error: false,
            elapsed_ms: Some(900),
            error: None,
        })
        .await;
    actor.sync().await;
    let work = actor.snapshot().background_work.remove("bg-1").unwrap();
    assert_eq!(work.description, "index files");
    assert_eq!(work.activity.as_deref(), Some("scanning src"));
    assert!(work.background);
    assert_eq!(work.status, "done");
    assert_eq!(work.elapsed_ms, Some(900));
    assert_eq!(work.error, None);
}

#[tokio::test]
async fn pending_tool_calls_deduplicated() {
    let actor = AgentStateActor::new();
    actor.add_pending_tool_call("a".into()).await;
    actor.add_pending_tool_call("a".into()).await;
    actor.sync().await;
    assert_eq!(actor.snapshot().pending_tool_calls.len(), 1);
}

#[tokio::test]
async fn event_projection_owns_stream_and_terminal_error_transitions() {
    let actor = AgentStateActor::new();
    let assistant = AgentMessage::Assistant(AssistantMessage {
        stop_reason: Some(StopReason::Aborted),
        error_message: Some("aborted".into()),
        ..Default::default()
    });
    actor
        .apply_event(&AgentEvent::MessageStart {
            message: assistant.clone(),
        })
        .await;
    actor.sync().await;
    assert!(actor.snapshot().is_streaming);
    actor
        .apply_event(&AgentEvent::MessageEnd { message: assistant })
        .await;
    actor.sync().await;
    let snapshot = actor.snapshot();
    assert!(snapshot.is_streaming);
    assert!(snapshot.error_message.is_none());
    assert_eq!(snapshot.messages.len(), 1);
    actor
        .apply_event(&AgentEvent::TurnEnd {
            message: AgentMessage::Assistant(AssistantMessage {
                error_message: Some("aborted".into()),
                ..Default::default()
            }),
            tool_results: vec![],
        })
        .await;
    actor.sync().await;
    assert_eq!(actor.snapshot().error_message.as_deref(), Some("aborted"));
    actor
        .apply_event(&AgentEvent::AgentEnd { messages: vec![] })
        .await;
    actor.sync().await;
    assert!(!actor.snapshot().is_streaming);
}

#[tokio::test]
async fn agent_start_reopens_stream_and_clears_previous_error() {
    let actor = AgentStateActor::new();
    actor.set_error(Some("previous failure".into())).await;
    actor.apply_event(&AgentEvent::AgentStart).await;
    actor.sync().await;
    let snapshot = actor.snapshot();
    assert!(snapshot.is_streaming);
    assert!(snapshot.streaming_message.is_none());
    assert!(snapshot.pending_tool_calls.is_empty());
    assert!(snapshot.error_message.is_none());
}

#[tokio::test]
async fn agent_end_clears_interrupted_pending_tool_calls() {
    let actor = AgentStateActor::new();
    actor.add_pending_tool_call("call-1".into()).await;
    actor
        .apply_event(&AgentEvent::AgentEnd { messages: vec![] })
        .await;
    actor.sync().await;
    assert!(actor.snapshot().pending_tool_calls.is_empty());
}

#[tokio::test]
async fn publish_event_keeps_bus_and_projection_on_one_event_boundary() {
    let actor = AgentStateActor::new();
    let bus = EventBus::new();
    let mut events = bus.subscribe();
    let message = AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "hey".into() }],
        timestamp: 1,
    });

    actor
        .publish_event(&bus, AgentEvent::MessageEnd { message })
        .await;
    actor.sync().await;

    assert!(matches!(
        events.try_recv(),
        Ok(AgentEvent::MessageEnd { .. })
    ));
    assert_eq!(actor.snapshot().messages.len(), 1);
}

#[tokio::test]
async fn error_event_owns_non_message_error_projection() {
    let actor = AgentStateActor::new();
    actor
        .apply_event(&AgentEvent::Error {
            message: "provider: no stream".into(),
        })
        .await;
    actor.sync().await;
    assert_eq!(
        actor.snapshot().error_message.as_deref(),
        Some("provider: no stream")
    );
}
