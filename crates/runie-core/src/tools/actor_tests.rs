use super::*;

struct ActorCancellationProbe {
    started: tokio::sync::watch::Sender<bool>,
}

#[async_trait::async_trait]
impl crate::types::AgentTool for ActorCancellationProbe {
    fn name(&self) -> &str {
        "actor_cancel_probe"
    }
    fn label(&self) -> &str {
        "actor_cancel_probe"
    }
    fn description(&self) -> &str {
        "actor cancellation test tool"
    }

    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<crate::types::AgentToolResult, String> {
        let _ = self.started.send(true);
        signal
            .expect("actor supplies cancellation")
            .cancelled()
            .await;
        Err("cancelled by actor probe".into())
    }
}

#[tokio::test]
async fn execute_empty_batch_completes() {
    let actor = ToolExecutorActor::new(Arc::new(ToolRegistry::new()));
    let outcome = execute_test_batch(&actor, vec![], ToolExecutionMode::Parallel).await;
    assert_empty_batch_outcome(outcome);
    let snapshot = actor.scheduler_metrics().await;
    assert_eq!(snapshot.completed, 1);
    assert_eq!(snapshot.running, 0);

    let _ = execute_test_batch(
        &actor,
        vec![ToolCall {
            id: "missing".into(),
            name: "missing_tool".into(),
            arguments: serde_json::json!({}),
            thought_signature: None,
        }],
        ToolExecutionMode::Sequential,
    )
    .await;
    let snapshot = actor.scheduler_metrics().await;
    assert_eq!(snapshot.completed, 1);
    assert_eq!(snapshot.failed, 1);
}

async fn execute_test_batch(
    actor: &ToolExecutorActor,
    calls: Vec<ToolCall>,
    mode: ToolExecutionMode,
) -> ToolOutcome {
    actor
        .execute(
            crate::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "test".into(),
                timestamp: 0,
                ..Default::default()
            },
            crate::types::AgentContext::default(),
            None,
            None,
            calls,
            mode,
            ToolExecHooks::default(),
        )
        .await
}

fn assert_empty_batch_outcome(outcome: ToolOutcome) {
    match outcome {
        ToolOutcome::Completed {
            tool_results,
            scheduler,
            ..
        } => {
            assert!(tool_results.is_empty());
            assert_eq!(scheduler.completed, 1);
            assert_eq!(scheduler.running, 0);
            assert_eq!(scheduler.interactive_enqueued, 1);
        }
        ToolOutcome::Aborted { .. } => panic!("expected completed"),
    }
}

#[tokio::test]
async fn aborting_actor_execution_projects_scheduler_cancellation() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ActorCancellationProbe {
        started: started_tx,
    }));
    let actor = ToolExecutorActor::new(Arc::new(registry));
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let execute = actor.execute(
        crate::types::AssistantMessage::default(),
        crate::types::AgentContext::default(),
        Some(abort_rx),
        None,
        vec![ToolCall {
            id: "cancel-1".into(),
            name: "actor_cancel_probe".into(),
            arguments: serde_json::json!({}),
            thought_signature: None,
        }],
        ToolExecutionMode::Sequential,
        ToolExecHooks::default(),
    );
    let abort = async {
        while !*started_rx.borrow() {
            let _ = started_rx.changed().await;
        }
        let _ = abort_tx.send(true);
    };
    let (outcome, ()) = tokio::join!(execute, abort);
    assert!(matches!(
        outcome,
        ToolOutcome::Completed {
            cancelled: true,
            ..
        }
    ));
    let snapshot = actor.scheduler_metrics().await;
    assert_eq!(snapshot.cancelled, 1);
    assert_eq!(snapshot.running, 0);
}

#[test]
fn dropped_executor_fallback_is_an_error_tool_result() {
    let calls = vec![ToolCall {
        id: "call-1".into(),
        name: "echo".into(),
        arguments: serde_json::json!({}),
        thought_signature: None,
    }];
    let ToolOutcome::Completed {
        tool_results,
        events,
        all_terminated,
        ..
    } = aborted_outcome(&calls, "executor dropped")
    else {
        panic!("fallback must complete with synthetic tool results");
    };
    assert!(!all_terminated);
    assert!(tool_results[0].is_error);
    assert_eq!(tool_results[0].tool_call_id, "call-1");
    assert!(matches!(
        events[1],
        AgentEvent::ToolExecutionEnd { is_error: true, .. }
    ));
}

#[tokio::test]
async fn queued_command_selection_prefers_interactive_work() {
    let (tx, mut rx) = mpsc::channel(4);
    let mut pending = Vec::new();
    for priority in [ToolPriority::Background, ToolPriority::Interactive] {
        let (reply, _) = oneshot::channel();
        tx.send(ToolCommand::Execute {
            assistant_message: AssistantMessage::default(),
            context: AgentContext::default(),
            abort: None,
            bus: None,
            calls: vec![],
            mode: ToolExecutionMode::Parallel,
            priority,
            hooks: ToolExecHooks::default(),
            reply,
        })
        .await
        .unwrap();
    }
    let selected = next_prioritized_command(&mut rx, &mut pending)
        .await
        .unwrap();
    assert!(matches!(
        selected,
        ToolCommand::Execute {
            priority: ToolPriority::Interactive,
            ..
        }
    ));
    assert_eq!(pending.len(), 1);
}

#[tokio::test]
async fn queued_command_cancellation_preserves_only_non_execution_commands() {
    let (reply, response) = oneshot::channel();
    let mut pending = vec![ToolCommand::Execute {
        assistant_message: AssistantMessage::default(),
        context: AgentContext::default(),
        abort: None,
        bus: None,
        calls: vec![],
        mode: ToolExecutionMode::Parallel,
        priority: ToolPriority::Background,
        hooks: ToolExecHooks::default(),
        reply,
    }];
    let mut metrics = SchedulerMetrics::default();
    assert_eq!(cancel_pending(&mut pending, &mut metrics), 1);
    assert!(pending.is_empty());
    assert_eq!(metrics.cancelled_queued, 1);
    let outcome = response.await.unwrap();
    assert!(matches!(outcome, ToolOutcome::Completed { .. }));
}
