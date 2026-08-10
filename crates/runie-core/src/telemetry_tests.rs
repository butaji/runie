use super::*;

struct RecordingExporter(tokio::sync::mpsc::UnboundedSender<TelemetrySnapshot>);

#[async_trait::async_trait]
impl TelemetryExporter for RecordingExporter {
    async fn export(&self, snapshot: TelemetrySnapshot) -> Result<(), String> {
        self.0
            .send(snapshot)
            .map_err(|_| "export receiver dropped".to_owned())
    }
}

struct FailingExporter;

#[async_trait::async_trait]
impl TelemetryExporter for FailingExporter {
    async fn export(&self, _snapshot: TelemetrySnapshot) -> Result<(), String> {
        Err("export backend unavailable".to_owned())
    }
}

#[test]
fn usage_summary_reduces_ended_provider_spans() {
    let snapshot = TelemetrySnapshot {
        spans: vec![SpanSnapshot {
            name: "pi.ai.request".into(),
            ended: true,
            attributes: [
                ("pi.ai.usage.input_tokens".into(), serde_json::json!(4)),
                ("pi.ai.usage.output_tokens".into(), serde_json::json!(6)),
                ("pi.ai.usage.total_tokens".into(), serde_json::json!(10)),
                ("pi.ai.usage.cost".into(), serde_json::json!(0.25)),
            ]
            .into_iter()
            .collect(),
            id: 1,
            parent_id: None,
            events: Vec::new(),
            status: SpanStatus::Ok,
            explicit_status: false,
            error: None,
            end_sequence: Some(1),
        }],
        ..TelemetrySnapshot::default()
    };
    let summary = usage_summary(&snapshot);
    assert_eq!(summary.requests, 1);
    assert_eq!(summary.total_tokens, 10);
    assert_eq!(summary.cost, 0.25);
}

#[tokio::test]
async fn nested_spans_and_terminal_state_are_actor_owned() {
    let actor = TelemetryActor::new();
    let root = actor.start_span(None, "run", HashMap::new()).await.unwrap();
    let child = root
        .with_child("request", HashMap::new(), |child| async move {
            child.event("headers", HashMap::new()).await;
            Ok::<_, &'static str>(child.id)
        })
        .await
        .unwrap()
        .unwrap();
    let sync_child = root
        .with_child_sync("sync-request", HashMap::new(), |child| {
            Ok::<_, &'static str>(child.id)
        })
        .await
        .unwrap()
        .unwrap();
    let snapshot = actor.snapshot();
    assert_eq!(root.id, 1);
    assert_eq!(snapshot.spans[1].parent_id, Some(root.id));
    assert_eq!(snapshot.spans[1].events[0].name, "headers");
    assert_eq!(snapshot.spans[1].status, SpanStatus::Ok);
    assert!(snapshot.spans[1].ended);
    assert_eq!(child, snapshot.spans[1].id);
    assert_eq!(sync_child, snapshot.spans[2].id);
    assert_eq!(snapshot.spans[2].parent_id, Some(root.id));
    assert!(snapshot.spans[2].ended);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn concurrent_child_callbacks_preserve_parentage_and_end_order() {
    let actor = TelemetryActor::new();
    let parent = actor
        .start_span(None, "parent", HashMap::new())
        .await
        .unwrap();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let first = parent.with_child("first-child", HashMap::new(), |child| async move {
        let _ = release_rx.await;
        Ok::<_, &'static str>(child.id)
    });
    tokio::pin!(first);
    let second = parent.with_child("second-child", HashMap::new(), |_child| async {
        Ok::<_, &'static str>(())
    });
    tokio::pin!(second);
    let second_result = tokio::select! {
        result = &mut second => result,
        _ = &mut first => panic!("first child cannot settle before its release"),
    };
    second_result
        .expect("second child callback")
        .expect("second child success");
    release_tx.send(()).expect("first child release");
    first
        .await
        .expect("first child callback")
        .expect("first child success");
    parent.end().await;

    assert_concurrent_span_order(&actor);
}

fn assert_concurrent_span_order(actor: &TelemetryActor) {
    let snapshot = actor.snapshot();
    let first = snapshot
        .spans
        .iter()
        .find(|span| span.name == "first-child")
        .unwrap();
    let second = snapshot
        .spans
        .iter()
        .find(|span| span.name == "second-child")
        .unwrap();
    let parent = snapshot
        .spans
        .iter()
        .find(|span| span.name == "parent")
        .unwrap();
    assert_eq!(first.parent_id, Some(parent.id));
    assert_eq!(second.parent_id, Some(parent.id));
    assert!(second.end_sequence < first.end_sequence);
    assert!(first.end_sequence < parent.end_sequence);
}

#[test]
fn pi_ai_request_schema_rejects_missing_invalid_and_unknown_operations() {
    let mut attributes = HashMap::from([
        ("pi.ai.operation".into(), serde_json::json!("stream")),
        ("pi.ai.provider".into(), serde_json::json!("openai")),
        ("pi.ai.model".into(), serde_json::json!("model")),
        ("pi.ai.api".into(), serde_json::json!("responses")),
        ("pi.ai.streaming".into(), serde_json::json!(true)),
    ]);
    assert!(validate_pi_ai_request_attributes(&attributes).is_ok());
    attributes.insert("pi.ai.operation".into(), serde_json::json!("unknown"));
    assert!(validate_pi_ai_request_attributes(&attributes).is_err());
    attributes.insert("pi.ai.operation".into(), serde_json::json!("stream"));
    attributes.insert("pi.ai.streaming".into(), serde_json::json!("true"));
    assert!(validate_pi_ai_request_attributes(&attributes).is_err());
    attributes.remove("pi.ai.model");
    assert!(validate_pi_ai_request_attributes(&attributes).is_err());
}

#[test]
fn pi_ai_request_end_schema_rejects_unknown_and_invalid_stop_reason() {
    let attributes = HashMap::from([
        (
            "pi.ai.response.stop_reason".into(),
            serde_json::json!("deferred"),
        ),
        ("pi.ai.usage.total_tokens".into(), serde_json::json!(12)),
    ]);
    assert!(validate_pi_ai_request_end_attributes(&attributes).is_ok());
    assert!(validate_pi_ai_request_end_attributes(&HashMap::from([(
        "pi.ai.response.stop_reason".into(),
        serde_json::json!("pending"),
    )]))
    .is_err());
    assert!(validate_pi_ai_request_end_attributes(&HashMap::from([(
        "pi.ai.unknown".into(),
        serde_json::json!(true),
    )]))
    .is_err());
}

#[test]
fn telemetry_attributes_accept_primitives_and_reject_mixed_json() {
    assert!(validate_telemetry_attributes(&HashMap::from([
        ("text".into(), serde_json::json!("value")),
        ("numbers".into(), serde_json::json!([1, 2])),
        ("flags".into(), serde_json::json!([true, false])),
    ])));
    assert!(!validate_telemetry_attributes(&HashMap::from([(
        "object".into(),
        serde_json::json!({"nested": true}),
    )])));
    assert!(!validate_telemetry_attributes(&HashMap::from([(
        "mixed".into(),
        serde_json::json!(["text", 1]),
    )])));
}

#[tokio::test]
async fn ended_spans_ignore_late_mutations() {
    let actor = TelemetryActor::new();
    let span = actor.start_span(None, "run", HashMap::new()).await.unwrap();
    span.end().await;
    span.event("late", HashMap::new()).await;
    assert!(span.child("late-child", HashMap::new()).await.is_none());
    assert!(actor.snapshot().spans[0].events.is_empty());
}

#[tokio::test]
async fn invalid_attribute_mutations_are_passive_and_atomic() {
    let actor = TelemetryActor::new();
    let span = actor
        .start_span(
            None,
            "run",
            HashMap::from([(String::from("kept"), serde_json::json!(true))]),
        )
        .await
        .unwrap();
    span.set_attributes(HashMap::from([
        (String::from("new"), serde_json::json!(1)),
        (String::from("invalid"), serde_json::json!({"nested": true})),
    ]))
    .await;
    span.event(
        "invalid",
        HashMap::from([(String::from("bad"), serde_json::json!(["x", 1]))]),
    )
    .await;
    let snapshot = actor.snapshot();
    assert_eq!(snapshot.spans[0].attributes.len(), 1);
    assert!(snapshot.spans[0].events.is_empty());
}

#[tokio::test]
async fn pi_request_schema_rejects_span_events_but_extension_spans_accept_them() {
    let actor = TelemetryActor::new();
    let request = actor
        .start_span(None, "pi.ai.request", HashMap::new())
        .await
        .unwrap();
    let extension = actor
        .start_span(None, "extension", HashMap::new())
        .await
        .unwrap();
    request.event("chunk", HashMap::new()).await;
    extension.event("chunk", HashMap::new()).await;
    let snapshot = actor.snapshot();
    assert!(snapshot.spans[0].events.is_empty());
    assert_eq!(snapshot.spans[1].events.len(), 1);
}

#[tokio::test]
async fn invalid_root_attributes_still_execute_noop_callback() {
    let actor = TelemetryActor::new();
    let result = actor
        .with_span(
            None,
            "ignored",
            HashMap::from([("nested".into(), serde_json::json!({"value": true}))]),
            |span| async move {
                assert_eq!(span.id, 0);
                Ok::<_, &'static str>("callback-ran")
            },
        )
        .await
        .expect("callback result")
        .expect("callback success");
    assert_eq!(result, "callback-ran");
    assert!(actor.snapshot().spans.is_empty());
}

#[tokio::test]
async fn settled_span_child_callback_remains_callable_without_recording() {
    let actor = TelemetryActor::new();
    let span = actor.start_span(None, "run", HashMap::new()).await.unwrap();
    span.end().await;
    let child = span
        .with_child("late-child", HashMap::new(), |child| async move {
            child.event("ignored", HashMap::new()).await;
            Ok::<_, &'static str>(child.id)
        })
        .await
        .expect("settled callback result")
        .expect("settled callback success");
    assert_eq!(child, 0);
    assert_eq!(actor.snapshot().spans.len(), 1);
}

#[tokio::test]
async fn callback_scoped_span_settles_success_and_error_through_actor() {
    let actor = TelemetryActor::new();
    let success = actor
        .with_span(None, "success", HashMap::new(), |span| async move {
            span.event("finished", HashMap::new()).await;
            Ok::<_, &'static str>("done")
        })
        .await
        .unwrap()
        .unwrap();
    let failure = actor
        .with_span(None, "failure", HashMap::new(), |_span| async {
            Err::<(), _>("failed")
        })
        .await
        .unwrap();
    assert_eq!(success, "done");
    assert_eq!(failure, Err("failed"));
    let snapshot = actor.snapshot();
    assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
    assert_eq!(snapshot.spans[1].status, SpanStatus::Error);
    assert_eq!(
        snapshot.spans[1].error,
        Some(SpanError {
            name: "Error".into(),
            message: "failed".into(),
        })
    );
    assert!(snapshot.spans.iter().all(|span| span.ended));
}

#[tokio::test]
async fn callback_scoped_failures_preserve_sync_and_async_error_values() {
    let actor = TelemetryActor::new();
    let sync = actor
        .with_span_sync(None, "sync-failure", HashMap::new(), |_span| {
            Err::<(), _>("sync error".to_owned())
        })
        .await
        .expect("sync callback result");
    let asynchronous = actor
        .with_span(None, "async-failure", HashMap::new(), |_span| async {
            Err::<(), _>("async error".to_owned())
        })
        .await
        .expect("async callback result");

    assert_eq!(sync, Err("sync error".to_owned()));
    assert_eq!(asynchronous, Err("async error".to_owned()));
    let snapshot = actor.snapshot();
    assert_eq!(snapshot.spans.len(), 2);
    assert!(snapshot.spans.iter().all(|span| span.ended));
    assert!(snapshot
        .spans
        .iter()
        .all(|span| span.status == SpanStatus::Error));
    assert_eq!(
        snapshot.spans[0].error.as_ref().unwrap().message,
        "sync error"
    );
    assert_eq!(
        snapshot.spans[1].error.as_ref().unwrap().message,
        "async error"
    );
}

#[tokio::test]
async fn explicit_status_survives_callback_failure() {
    let actor = TelemetryActor::new();
    let result = actor
        .with_span(None, "explicit", HashMap::new(), |span| async move {
            span.status(SpanStatus::Ok).await;
            Err::<(), _>("failure")
        })
        .await
        .expect("callback result");
    assert_eq!(result, Err("failure"));
    let snapshot = actor.snapshot();
    assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
    assert!(snapshot.spans[0].explicit_status);
}

#[tokio::test]
async fn active_span_defaults_to_pi_ok_status() {
    let actor = TelemetryActor::new();
    let span = actor
        .start_span(None, "active", HashMap::new())
        .await
        .unwrap();
    assert_eq!(actor.snapshot().spans[0].status, SpanStatus::Ok);
    span.end().await;
}

#[tokio::test]
async fn settled_span_is_exported_after_actor_reduction() {
    let (export_tx, mut export_rx) = tokio::sync::mpsc::unbounded_channel();
    let actor = TelemetryActor::new_with_exporter(Some(Arc::new(RecordingExporter(export_tx))));
    let span = actor
        .start_span(None, "exported", HashMap::new())
        .await
        .unwrap();
    span.end().await;
    let exported = export_rx.recv().await.expect("settled export");
    assert_eq!(exported.spans.len(), 1);
    assert!(exported.spans[0].ended);
    assert_eq!(exported.spans[0].name, "exported");
}

#[tokio::test]
async fn exporter_failure_does_not_rewind_settled_actor_state() {
    let actor = TelemetryActor::new_with_exporter(Some(Arc::new(FailingExporter)));
    let span = actor
        .start_span(None, "export-failure", HashMap::new())
        .await
        .unwrap();
    span.end().await;

    let snapshot = actor.snapshot();
    assert_eq!(snapshot.spans.len(), 1);
    assert!(snapshot.spans[0].ended);
    assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
    assert_eq!(snapshot.spans[0].name, "export-failure");
}
