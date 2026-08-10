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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub cost: f64,
}

pub fn usage_summary(snapshot: &TelemetrySnapshot) -> UsageSummary {
    snapshot
        .spans
        .iter()
        .filter(|span| span.ended && span.name == "pi.ai.request")
        .fold(UsageSummary::default(), |mut summary, span| {
            summary.requests += 1;
            summary.input_tokens += attribute_u64(span, "pi.ai.usage.input_tokens");
            summary.output_tokens += attribute_u64(span, "pi.ai.usage.output_tokens");
            summary.cache_read_tokens += attribute_u64(span, "pi.ai.usage.cache_read_tokens");
            summary.cache_write_tokens += attribute_u64(span, "pi.ai.usage.cache_write_tokens");
            summary.reasoning_tokens += attribute_u64(span, "pi.ai.usage.reasoning_tokens");
            summary.total_tokens += attribute_u64(span, "pi.ai.usage.total_tokens");
            summary.cost += span
                .attributes
                .get("pi.ai.usage.cost")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or_default();
            summary
        })
}

fn attribute_u64(span: &SpanSnapshot, key: &str) -> u64 {
    span.attributes
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
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
        validate_end_attribute(key, value, &STRING_KEYS, &NUMBER_KEYS)?;
    }
    Ok(())
}

fn validate_end_attribute(
    key: &str,
    value: &serde_json::Value,
    strings: &[&str],
    numbers: &[&str],
) -> Result<(), String> {
    if strings.contains(&key) {
        return validate_end_string(key, value);
    }
    if numbers.contains(&key) || key == "pi.ai.stream.time_to_first_chunk_ms" {
        return value
            .is_number()
            .then_some(())
            .ok_or_else(|| format!("Pi telemetry {key} must be a number"));
    }
    Err(format!("unknown Pi telemetry end attribute {key}"))
}

fn validate_end_string(key: &str, value: &serde_json::Value) -> Result<(), String> {
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
    Ok(())
}

/// Pi telemetry attributes are deliberately narrower than arbitrary JSON:
/// values are primitives or homogeneous primitive arrays. Invalid payloads
/// are passive and the containing mutation is ignored atomically.
pub fn validate_telemetry_attributes(attributes: &HashMap<String, serde_json::Value>) -> bool {
    attributes.values().all(is_telemetry_attribute)
}

fn is_telemetry_attribute(value: &serde_json::Value) -> bool {
    value.is_string()
        || value.is_number()
        || value.is_boolean()
        || value
            .as_array()
            .is_some_and(|values| is_homogeneous_primitive_array(values))
}

fn is_homogeneous_primitive_array(values: &[serde_json::Value]) -> bool {
    values.iter().all(is_primitive_or_empty_null)
        && values
            .windows(2)
            .all(|pair| primitive_kind(&pair[0]) == primitive_kind(&pair[1]))
}

fn is_primitive_or_empty_null(value: &serde_json::Value) -> bool {
    value.is_string() || value.is_number() || value.is_boolean()
}

fn primitive_kind(value: &serde_json::Value) -> u8 {
    if value.is_string() {
        0
    } else if value.is_number() {
        1
    } else {
        2
    }
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
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<TelemetrySnapshot>>,
    _owner: Arc<TaskOwner>,
}

#[path = "telemetry_runtime.rs"]
mod telemetry_runtime;
#[path = "telemetry_span.rs"]
mod telemetry_span;
#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
pub use telemetry_span::TelemetrySpan;
