use super::*;
use crate::types::{AfterToolCallContext, AfterToolCallResult, AgentTool};

struct CancellationProbe {
    started: tokio::sync::watch::Sender<bool>,
}

struct AbortAwareTool {
    executed: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl AgentTool for AbortAwareTool {
    fn name(&self) -> &str {
        "abort_aware"
    }
    fn label(&self) -> &str {
        "Abort-aware tool"
    }
    fn description(&self) -> &str {
        "Records whether execution was reached."
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.executed.store(true, Ordering::Release);
        Ok(AgentToolResult::default())
    }
}

#[async_trait::async_trait]
impl AgentTool for CancellationProbe {
    fn name(&self) -> &str {
        "cancel_probe"
    }

    fn label(&self) -> &str {
        "Cancellation probe"
    }

    fn description(&self) -> &str {
        "Waits for cancellation."
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        _args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        if let Some(on_update) = on_update {
            on_update(serde_json::json!({"phase": "started"}));
        }
        let _ = self.started.send(true);
        signal
            .expect("executor must provide a cancellation token")
            .cancelled()
            .await;
        Err("cancelled by probe".into())
    }
}

#[test]
fn empty_sequential_produces_no_results() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let registry = Arc::new(ToolRegistry::new());
        let ctx = ToolExecContext {
            assistant_message: AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "test".into(),
                timestamp: 0,
                ..Default::default()
            },
            context: crate::types::AgentContext::default(),
            abort: None,
            registry,
            hooks: ToolExecHooks::default(),
            bus: None,
            updates: Some(Arc::new(std::sync::Mutex::new(Vec::new()))),
            tool_result_timestamp: 0,
        };
        let outcome = execute_sequential(vec![], ctx).await;
        assert!(outcome.tool_results.is_empty());
        assert!(!outcome.all_terminated);
    });
}

#[test]
fn settled_tool_update_callbacks_are_ignored() {
    let settled = AtomicBool::new(false);
    assert!(tool_update_is_live(&settled));
    settled.store(true, Ordering::Release);
    assert!(!tool_update_is_live(&settled));
}

#[tokio::test]
async fn already_aborted_call_matches_pi_before_tool_boundary() {
    let executed = Arc::new(AtomicBool::new(false));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(AbortAwareTool {
        executed: executed.clone(),
    }));
    let (_abort_tx, abort_rx) = tokio::sync::watch::channel(true);
    let hook_saw_aborted = Arc::new(AtomicBool::new(false));
    let hook_saw_aborted_clone = hook_saw_aborted.clone();
    let ctx = ToolExecContext {
        assistant_message: AssistantMessage::default(),
        context: crate::types::AgentContext::default(),
        abort: Some(abort_rx),
        registry: Arc::new(registry),
        hooks: ToolExecHooks {
            before_tool_call: Some(Arc::new(move |input| {
                hook_saw_aborted_clone.store(input.signal.is_cancelled(), Ordering::Release);
                Box::pin(async { BeforeToolCallResult::default() })
            })),
            ..ToolExecHooks::default()
        },
        bus: None,
        tool_result_timestamp: 0,
        updates: Some(Arc::new(std::sync::Mutex::new(Vec::new()))),
    };
    let call = ToolCall {
        id: "abort-1".into(),
        name: "abort_aware".into(),
        arguments: serde_json::json!({}),
        thought_signature: None,
    };

    let error = dispatch_one(&call, &ctx).await.expect_err("aborted call");
    assert_eq!(error, "Operation aborted");
    assert!(hook_saw_aborted.load(Ordering::Acquire));
    assert!(!executed.load(Ordering::Acquire));
}

#[tokio::test]
async fn abort_during_before_tool_hook_cancels_hook_signal() {
    let executed = Arc::new(AtomicBool::new(false));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(AbortAwareTool {
        executed: executed.clone(),
    }));
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let ctx = ToolExecContext {
        assistant_message: AssistantMessage::default(),
        context: crate::types::AgentContext::default(),
        abort: Some(abort_rx),
        registry: Arc::new(registry),
        hooks: ToolExecHooks {
            before_tool_call: Some(Arc::new(move |input| {
                let abort_tx = abort_tx.clone();
                Box::pin(async move {
                    abort_tx.send(true).expect("abort receiver is live");
                    input.signal.cancelled().await;
                    BeforeToolCallResult::default()
                })
            })),
            ..ToolExecHooks::default()
        },
        bus: None,
        tool_result_timestamp: 0,
        updates: Some(Arc::new(std::sync::Mutex::new(Vec::new()))),
    };
    let call = abort_hook_call();

    let error = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        dispatch_one(&call, &ctx),
    )
    .await
    .expect("abort should settle the hook")
    .expect_err("aborted call");
    assert_eq!(error, "Operation aborted");
    assert!(!executed.load(Ordering::Acquire));
}

fn abort_hook_call() -> ToolCall {
    ToolCall {
        id: "abort-hook-1".into(),
        name: "abort_aware".into(),
        arguments: serde_json::json!({}),
        thought_signature: None,
    }
}

#[tokio::test]
async fn abort_cancels_in_flight_tool_and_emits_error_result() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CancellationProbe {
        started: started_tx,
    }));
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let bus = crate::events::EventBus::new();
    let mut bus_events = bus.subscribe();
    let ctx = ToolExecContext {
        assistant_message: AssistantMessage::default(),
        context: crate::types::AgentContext::default(),
        abort: Some(abort_rx),
        bus: Some(bus),
        tool_result_timestamp: 0,
        registry: Arc::new(registry),
        hooks: ToolExecHooks {
            after_tool_call: Some(Arc::new(|input| {
                Box::pin(async move {
                    assert!(input.is_error);
                    crate::types::AfterToolCallResult::default()
                })
            })),
            ..ToolExecHooks::default()
        },
        updates: Some(Arc::new(std::sync::Mutex::new(Vec::new()))),
    };
    let call = cancellation_call();
    let abort_when_started = async {
        while !*started_rx.borrow() {
            let _ = started_rx.changed().await;
        }
        let _ = abort_tx.send(true);
    };
    let (outcome, _) = tokio::join!(execute_sequential(vec![call], ctx), abort_when_started);
    assert_abort_outcome(outcome, &mut bus_events);
}

fn cancellation_call() -> ToolCall {
    ToolCall {
        id: "cancel-1".into(),
        name: "cancel_probe".into(),
        arguments: serde_json::json!({}),
        thought_signature: None,
    }
}

fn assert_abort_outcome(
    outcome: DispatchOutcome,
    bus_events: &mut tokio::sync::broadcast::Receiver<crate::types::AgentEvent>,
) {
    let live_update = bus_events
        .try_recv()
        .expect("tool update should publish before completion");
    assert!(matches!(
        live_update,
        crate::types::AgentEvent::ToolExecutionUpdate { .. }
    ));
    assert!(outcome.tool_results[0].is_error);
    assert_eq!(outcome.tool_results[0].details, serde_json::json!({}));
    assert!(matches!(
        outcome.tool_results[0].content.first(),
        Some(ToolResultContent::Text { text }) if text == "aborted"
    ));
}

#[allow(dead_code)]
fn _after_ctx_marker(_c: AfterToolCallContext) -> AfterToolCallResult {
    AfterToolCallResult::default()
}
