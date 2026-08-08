//! Actor-owned telemetry capability for Pi-compatible span lifecycles.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use crate::task_owner::{spawn_actor_worker, TaskOwner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanError {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSnapshot {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<TelemetryEventSnapshot>,
    pub status: SpanStatus,
    #[serde(default)]
    pub explicit_status: bool,
    #[serde(default)]
    pub error: Option<SpanError>,
    pub ended: bool,
    #[serde(default)]
    pub end_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryEventSnapshot {
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub next_id: u64,
    pub spans: Vec<SpanSnapshot>,
}

/// Validate the source-defined start vocabulary for Pi's `pi.ai.request`
/// span. The generic actor remains schema-agnostic so extension spans can be
/// recorded, while provider adapters can opt into this typed boundary.
pub fn validate_pi_ai_request_attributes(
    attributes: &HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    const REQUIRED: [&str; 5] = [
        "pi.ai.operation",
        "pi.ai.provider",
        "pi.ai.model",
        "pi.ai.api",
        "pi.ai.streaming",
    ];
    for key in REQUIRED {
        if !attributes.contains_key(key) {
            return Err(format!("missing Pi telemetry attribute {key}"));
        }
    }
    let operation = attributes["pi.ai.operation"]
        .as_str()
        .ok_or_else(|| "Pi telemetry pi.ai.operation must be a string".to_owned())?;
    if !matches!(
        operation,
        "stream" | "fetch_deferred" | "cancel_deferred" | "generate_images"
    ) {
        return Err(format!("unsupported Pi telemetry operation {operation}"));
    }
    for key in ["pi.ai.provider", "pi.ai.model", "pi.ai.api"] {
        if !attributes[key].is_string() {
            return Err(format!("Pi telemetry {key} must be a string"));
        }
    }
    if !attributes["pi.ai.streaming"].is_boolean() {
        return Err("Pi telemetry pi.ai.streaming must be a boolean".to_owned());
    }
    if let Some(deferred) = attributes.get("pi.ai.deferred") {
        if !deferred.is_boolean() {
            return Err("Pi telemetry pi.ai.deferred must be a boolean".to_owned());
        }
    }
    Ok(())
}

/// Validate the source-defined end vocabulary for Pi's `pi.ai.request` span.
/// End attributes are optional, but known fields must retain Pi's primitive
/// types and closed stop-reason values.
#[allow(
    clippy::too_many_lines,
    reason = "the Pi schema keeps its closed end vocabulary together at one typed boundary"
)]
pub fn validate_pi_ai_request_end_attributes(
    attributes: &HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    const STRING_KEYS: [&str; 4] = [
        "pi.ai.response.model",
        "pi.ai.response.id",
        "pi.ai.error.type",
        "pi.ai.response.stop_reason",
    ];
    const NUMBER_KEYS: [&str; 9] = [
        "pi.ai.http.status_code",
        "pi.ai.usage.input_tokens",
        "pi.ai.usage.output_tokens",
        "pi.ai.usage.cache_read_tokens",
        "pi.ai.usage.cache_write_tokens",
        "pi.ai.usage.reasoning_tokens",
        "pi.ai.usage.total_tokens",
        "pi.ai.usage.cost",
        "pi.ai.stream.chunk_count",
    ];
    for (key, value) in attributes {
        if STRING_KEYS.contains(&key.as_str()) {
            if !value.is_string() {
                return Err(format!("Pi telemetry {key} must be a string"));
            }
            if key == "pi.ai.response.stop_reason"
                && !matches!(
                    value.as_str(),
                    Some("stop" | "length" | "tool_use" | "error" | "aborted" | "deferred")
                )
            {
                return Err("Pi telemetry response stop reason is not supported".to_owned());
            }
        } else if NUMBER_KEYS.contains(&key.as_str())
            || key == "pi.ai.stream.time_to_first_chunk_ms"
        {
            if !value.is_number() {
                return Err(format!("Pi telemetry {key} must be a number"));
            }
        } else {
            return Err(format!("unknown Pi telemetry end attribute {key}"));
        }
    }
    Ok(())
}

/// Pi telemetry attributes are deliberately narrower than arbitrary JSON:
/// values are primitives or homogeneous primitive arrays. Invalid payloads
/// are passive and the containing mutation is ignored atomically.
pub fn validate_telemetry_attributes(attributes: &HashMap<String, serde_json::Value>) -> bool {
    attributes.values().all(is_telemetry_attribute)
}

fn is_telemetry_attribute(value: &serde_json::Value) -> bool {
    if value.is_string() || value.is_number() || value.is_boolean() {
        return true;
    }
    let Some(values) = value.as_array() else {
        return false;
    };
    values.iter().all(|item| {
        item.is_string()
            || item.is_number()
            || item.is_boolean()
            || item.is_null() && values.is_empty()
    }) && (values.is_empty() || {
        let kind = |item: &serde_json::Value| {
            if item.is_string() {
                0
            } else if item.is_number() {
                1
            } else {
                2
            }
        };
        values
            .windows(2)
            .all(|pair| kind(&pair[0]) == kind(&pair[1]))
    })
}

#[async_trait::async_trait]
pub trait TelemetryExporter: Send + Sync + 'static {
    async fn export(&self, snapshot: TelemetrySnapshot) -> Result<(), String>;
}

/// Runtime-editable telemetry replay commands. These address the actor
/// mailbox contract and never expose reducer state to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryAction {
    Start {
        #[serde(default)]
        parent_id: Option<u64>,
        name: String,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },
    Event {
        id: u64,
        name: String,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },
    SetAttributes {
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
    },
    Status {
        id: u64,
        status: SpanStatus,
        #[serde(default)]
        error: Option<SpanError>,
    },
    End {
        id: u64,
    },
}

/// A complete actor replay case, loaded from YAML without recompilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryScenario {
    pub actions: Vec<TelemetryAction>,
    pub expected: TelemetrySnapshot,
}

enum TelemetryCommand {
    Start {
        parent_id: Option<u64>,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<Option<u64>>,
    },
    Event {
        id: u64,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    },
    SetAttributes {
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    },
    Status {
        id: u64,
        status: SpanStatus,
        error: Option<SpanError>,
        reply: oneshot::Sender<()>,
    },
    End {
        id: u64,
        reply: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct TelemetryActor {
    tx: mpsc::Sender<TelemetryCommand>,
    snapshot: watch::Receiver<TelemetrySnapshot>,
    _owner: Arc<TaskOwner>,
}

impl TelemetryActor {
    #[allow(
        clippy::too_many_lines,
        reason = "the actor reducer keeps the complete span lifecycle in one explicit boundary"
    )]
    pub fn new() -> Self {
        Self::new_with_exporter(None)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the telemetry actor keeps its complete reducer and exporter settlement boundary together"
    )]
    pub fn new_with_exporter(exporter: Option<Arc<dyn TelemetryExporter>>) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(TelemetrySnapshot::default());
        let (tx, owner) = spawn_actor_worker!(32, move |mut rx: mpsc::Receiver<
            TelemetryCommand,
        >| async move {
            let mut state = TelemetrySnapshot {
                next_id: 1,
                ..TelemetrySnapshot::default()
            };
            let mut next_end_sequence = 0_u64;
            while let Some(command) = rx.recv().await {
                match command {
                    TelemetryCommand::Start {
                        parent_id,
                        name,
                        attributes,
                        reply,
                    } => {
                        if !validate_telemetry_attributes(&attributes) {
                            let _ = reply.send(None);
                            continue;
                        }
                        if parent_id.is_some_and(|parent_id| {
                            !state
                                .spans
                                .iter()
                                .any(|span| span.id == parent_id && !span.ended)
                        }) {
                            let _ = reply.send(None);
                            continue;
                        }
                        let id = state.next_id;
                        state.next_id = state.next_id.wrapping_add(1);
                        state.spans.push(SpanSnapshot {
                            id,
                            parent_id,
                            name,
                            attributes,
                            events: Vec::new(),
                            status: SpanStatus::Ok,
                            explicit_status: false,
                            error: None,
                            ended: false,
                            end_sequence: None,
                        });
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Some(id));
                    }
                    TelemetryCommand::Event {
                        id,
                        name,
                        attributes,
                        reply,
                    } => {
                        if !validate_telemetry_attributes(&attributes) {
                            let _ = reply.send(());
                            continue;
                        }
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.events
                                .push(TelemetryEventSnapshot { name, attributes });
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::SetAttributes {
                        id,
                        attributes,
                        reply,
                    } => {
                        if !validate_telemetry_attributes(&attributes) {
                            let _ = reply.send(());
                            continue;
                        }
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.attributes.extend(attributes);
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::Status {
                        id,
                        status,
                        error,
                        reply,
                    } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.status = status;
                            span.error = error;
                            span.explicit_status = true;
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::End { id, reply } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.ended = true;
                            span.end_sequence = Some(next_end_sequence);
                            next_end_sequence = next_end_sequence.wrapping_add(1);
                            let _ = snapshot_tx.send(state.clone());
                            if let Some(exporter) = exporter.as_ref() {
                                let _ = exporter.export(state.clone()).await;
                            }
                        }
                        let _ = reply.send(());
                    }
                }
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        self.snapshot.borrow().clone()
    }

    /// Apply one declarative replay action through the actor mailbox.
    #[allow(
        clippy::too_many_lines,
        reason = "the replay DSL maps each declared actor command explicitly"
    )]
    pub async fn apply(&self, action: TelemetryAction) -> Option<u64> {
        match action {
            TelemetryAction::Start {
                parent_id,
                name,
                attributes,
            } => self
                .start_span(parent_id, name, attributes)
                .await
                .map(|span| span.id),
            TelemetryAction::Event {
                id,
                name,
                attributes,
            } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                    noop: false,
                }
                .event(name, attributes)
                .await;
                None
            }
            TelemetryAction::SetAttributes { id, attributes } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                    noop: false,
                }
                .set_attributes(attributes)
                .await;
                None
            }
            TelemetryAction::Status { id, status, error } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                    noop: false,
                }
                .status_with_error(status, error)
                .await;
                None
            }
            TelemetryAction::End { id } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                    noop: false,
                }
                .end()
                .await;
                None
            }
        }
    }

    pub async fn replay(&self, actions: impl IntoIterator<Item = TelemetryAction>) {
        for action in actions {
            let _ = self.apply(action).await;
        }
    }

    pub async fn start_span(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<TelemetrySpan> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(TelemetryCommand::Start {
                parent_id,
                name: name.into(),
                attributes,
                reply,
            })
            .await
            .ok()?;
        Some(TelemetrySpan {
            actor: self.clone(),
            id: result.await.ok()??,
            noop: false,
        })
    }

    /// Execute a callback inside an actor-owned span, matching Pi's
    /// callback-scoped `startSpan` contract. Completion always settles the
    /// span through mailbox commands before the result is returned.
    pub async fn with_span<F, Fut, T, E>(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let span = self
            .start_span(parent_id, name, attributes)
            .await
            .unwrap_or_else(|| TelemetrySpan {
                actor: self.clone(),
                id: 0,
                noop: true,
            });
        let result = callback(span.clone()).await;
        if !span.noop {
            match &result {
                Ok(_) => span.status(SpanStatus::Ok).await,
                Err(error) if !span.explicit_status() => {
                    span.status_with_error(
                        SpanStatus::Error,
                        Some(SpanError {
                            name: "Error".to_owned(),
                            message: error.to_string(),
                        }),
                    )
                    .await;
                }
                Err(_) => {}
            }
            span.end().await;
        }
        Some(result)
    }
}

impl Default for TelemetryActor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TelemetrySpan {
    actor: TelemetryActor,
    pub id: u64,
    noop: bool,
}

impl TelemetrySpan {
    fn explicit_status(&self) -> bool {
        self.actor
            .snapshot()
            .spans
            .iter()
            .find(|span| span.id == self.id)
            .is_some_and(|span| span.explicit_status)
    }

    pub async fn event(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Event {
                id: self.id,
                name: name.into(),
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn set_attributes(&self, attributes: HashMap<String, serde_json::Value>) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::SetAttributes {
                id: self.id,
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn status(&self, status: SpanStatus) {
        self.status_with_error(status, None).await;
    }

    pub async fn status_with_error(&self, status: SpanStatus, error: Option<SpanError>) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Status {
                id: self.id,
                status,
                error,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn end(&self) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::End { id: self.id, reply })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn child(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<Self> {
        self.actor.start_span(Some(self.id), name, attributes).await
    }

    /// Run a nested callback-scoped span through the owning actor.
    pub async fn with_child<F, Fut, T, E>(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let child = self.actor.start_span(Some(self.id), name, attributes).await;
        // Pi's settled-span context remains callable: the callback executes,
        // but all operations on the detached child are inert. Preserve that
        // behavior without creating a second recorded span.
        let span = child.unwrap_or_else(|| TelemetrySpan {
            actor: self.actor.clone(),
            id: 0,
            noop: true,
        });
        let result = callback(span.clone()).await;
        if !span.noop {
            match &result {
                Ok(_) => span.status(SpanStatus::Ok).await,
                Err(error) if !span.explicit_status() => {
                    span.status_with_error(
                        SpanStatus::Error,
                        Some(SpanError {
                            name: "Error".to_owned(),
                            message: error.to_string(),
                        }),
                    )
                    .await;
                }
                Err(_) => {}
            }
            span.end().await;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
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
        let snapshot = actor.snapshot();
        assert_eq!(root.id, 1);
        assert_eq!(snapshot.spans[1].parent_id, Some(root.id));
        assert_eq!(snapshot.spans[1].events[0].name, "headers");
        assert_eq!(snapshot.spans[1].status, SpanStatus::Ok);
        assert!(snapshot.spans[1].ended);
        assert_eq!(child, snapshot.spans[1].id);
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
}
